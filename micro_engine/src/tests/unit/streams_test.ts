// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { assertEquals, fail } from "./test_util.ts";

const {
  core,
  resourceForReadableStream,
  // @ts-expect-error TypeScript (as of 3.7) does not support indexing namespaces by symbol
} = Deno[Deno.internal];

const LOREM =
  "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";

// Hello world, with optional close
function helloWorldStream(
  close?: boolean,
  cancelResolve?: (value: unknown) => void,
) {
  return new ReadableStream({
    start(controller) {
      controller.enqueue("hello, world");
      if (close == true) {
        controller.close();
      }
    },
    cancel(reason) {
      if (cancelResolve != undefined) {
        cancelResolve(reason);
      }
    },
  }).pipeThrough(new TextEncoderStream());
}

// Hello world, with optional close
function errorStream(type: "string" | "controller" | "TypeError") {
  return new ReadableStream({
    start(controller) {
      controller.enqueue("hello, world");
    },
    pull(controller) {
      if (type == "string") {
        throw "Uh oh (string)!";
      }
      if (type == "TypeError") {
        throw TypeError("Uh oh (TypeError)!");
      }
      controller.error("Uh oh (controller)!");
    },
  }).pipeThrough(new TextEncoderStream());
}

// Long stream with Lorem Ipsum text.
function longStream() {
  return new ReadableStream({
    start(controller) {
      for (let i = 0; i < 4; i++) {
        setTimeout(() => {
          controller.enqueue(LOREM);
          if (i == 3) {
            controller.close();
          }
        }, i * 100);
      }
    },
  }).pipeThrough(new TextEncoderStream());
}

// Long stream with Lorem Ipsum text.
function longAsyncStream(cancelResolve?: (value: unknown) => void) {
  let currentTimeout: number | undefined = undefined;
  return new ReadableStream({
    async start(controller) {
      for (let i = 0; i < 100; i++) {
        await new Promise((r) => currentTimeout = setTimeout(r, 1));
        currentTimeout = undefined;
        controller.enqueue(LOREM);
      }
      controller.close();
    },
    cancel(reason) {
      if (cancelResolve != undefined) {
        cancelResolve(reason);
      }
      if (currentTimeout !== undefined) {
        clearTimeout(currentTimeout);
      }
    },
  }).pipeThrough(new TextEncoderStream());
}

// Empty stream, closes either immediately or on a call to pull.
function emptyStream(onPull: boolean) {
  return new ReadableStream({
    start(controller) {
      if (!onPull) {
        controller.close();
      }
    },
    pull(controller) {
      if (onPull) {
        controller.close();
      }
    },
  }).pipeThrough(new TextEncoderStream());
}

function largePacketStream(packetSize: number, count: number) {
  return new ReadableStream({
    pull(controller) {
      if (count-- > 0) {
        const buffer = new Uint8Array(packetSize);
        for (let i = 0; i < 256; i++) {
          buffer[i * (packetSize / 256)] = i;
        }
        controller.enqueue(buffer);
      } else {
        controller.close();
      }
    },
  });
}

// Include an empty chunk
function emptyChunkStream() {
  return new ReadableStream({
    start(controller) {
      controller.enqueue(new Uint8Array([1]));
      controller.enqueue(new Uint8Array([]));
      controller.enqueue(new Uint8Array([2]));
      controller.close();
    },
  });
}

// Try to blow up any recursive reads.
function veryLongTinyPacketStream(length: number) {
  return new ReadableStream({
    start(controller) {
      for (let i = 0; i < length; i++) {
        controller.enqueue(new Uint8Array([1]));
      }
      controller.close();
    },
  });
}

// Creates a stream with the given number of packets, a configurable delay between packets, and a final
// action (either "Throw" or "Close").
function makeStreamWithCount(
  count: number,
  delay: number,
  action: "Throw" | "Close",
): ReadableStream {
  function doAction(controller: ReadableStreamDefaultController, i: number) {
    if (i == count) {
      if (action == "Throw") {
        controller.error(new Error("Expected error!"));
      } else {
        controller.close();
      }
    } else {
      controller.enqueue(String.fromCharCode("a".charCodeAt(0) + i));

      if (delay == 0) {
        doAction(controller, i + 1);
      } else {
        setTimeout(() => doAction(controller, i + 1), delay);
      }
    }
  }

  return new ReadableStream({
    start(controller) {
      if (delay == 0) {
        doAction(controller, 0);
      } else {
        setTimeout(() => doAction(controller, 0), delay);
      }
    },
  }).pipeThrough(new TextEncoderStream());
}

// Normal stream operation
Deno.test(async function readableStream() {
  const rid = resourceForReadableStream(helloWorldStream());
  const buffer = new Uint8Array(1024);
  const nread = await core.ops.op_read(rid, buffer);
  assertEquals(nread, 12);
  core.ops.op_close(rid);
});

// Close the stream after reading everything
Deno.test(async function readableStreamClose() {
  const { promise: cancelPromise, resolve: cancelResolve } = Promise
    .withResolvers();
  const rid = resourceForReadableStream(helloWorldStream(false, cancelResolve));
  const buffer = new Uint8Array(1024);
  const nread = await core.ops.op_read(rid, buffer);
  assertEquals(nread, 12);
  core.ops.op_close(rid);
  assertEquals(await cancelPromise, "resource closed");
});

// Close the stream without reading everything
Deno.test(async function readableStreamClosePartialRead() {
  const { promise: cancelPromise, resolve: cancelResolve } = Promise
    .withResolvers();
  const rid = resourceForReadableStream(helloWorldStream(false, cancelResolve));
  const buffer = new Uint8Array(5);
  const nread = await core.ops.op_read(rid, buffer);
  assertEquals(nread, 5);
  core.ops.op_close(rid);
  assertEquals(await cancelPromise, "resource closed");
});

// Close the stream without reading anything
Deno.test(async function readableStreamCloseWithoutRead() {
  const { promise: cancelPromise, resolve: cancelResolve } = Promise
    .withResolvers();
  const rid = resourceForReadableStream(helloWorldStream(false, cancelResolve));
  core.ops.op_close(rid);
  assertEquals(await cancelPromise, "resource closed");
});

// Close the stream without reading anything
Deno.test(async function readableStreamCloseWithoutRead2() {
  const { promise: cancelPromise, resolve: cancelResolve } = Promise
    .withResolvers();
  const rid = resourceForReadableStream(longAsyncStream(cancelResolve));
  core.ops.op_close(rid);
  assertEquals(await cancelPromise, "resource closed");
});

Deno.test(async function readableStreamPartial() {
  const rid = resourceForReadableStream(helloWorldStream());
  const buffer = new Uint8Array(5);
  const nread = await core.ops.op_read(rid, buffer);
  assertEquals(nread, 5);
  const buffer2 = new Uint8Array(1024);
  const nread2 = await core.ops.op_read(rid, buffer2);
  assertEquals(nread2, 7);
  core.ops.op_close(rid);
});

Deno.test(async function readableStreamLongReadAll() {
  const rid = resourceForReadableStream(longStream());
  const buffer = await core.ops.op_read_all(rid);
  assertEquals(buffer.length, LOREM.length * 4);
  core.ops.op_close(rid);
});

Deno.test(async function readableStreamLongAsyncReadAll() {
  const rid = resourceForReadableStream(longAsyncStream());
  const buffer = await core.ops.op_read_all(rid);
  assertEquals(buffer.length, LOREM.length * 100);
  core.ops.op_close(rid);
});

Deno.test(async function readableStreamVeryLongReadAll() {
  const rid = resourceForReadableStream(veryLongTinyPacketStream(1_000_000));
  const buffer = await core.ops.op_read_all(rid);
  assertEquals(buffer.length, 1_000_000);
  core.ops.op_close(rid);
});

Deno.test(async function readableStreamLongByPiece() {
  const rid = resourceForReadableStream(longStream());
  let total = 0;
  for (let i = 0; i < 100; i++) {
    const length = await core.ops.op_read(rid, new Uint8Array(16));
    total += length;
    if (length == 0) {
      break;
    }
  }
  assertEquals(total, LOREM.length * 4);
  core.ops.op_close(rid);
});

for (
  const type of [
    "string",
    "TypeError",
    "controller",
  ] as ("string" | "TypeError" | "controller")[]
) {
  Deno.test(`readableStreamError_${type}`, async function () {
    const rid = resourceForReadableStream(errorStream(type));
    let nread;
    try {
      nread = await core.ops.op_read(rid, new Uint8Array(16));
    } catch (_) {
      fail("Should not have thrown");
    }
    assertEquals(12, nread);
    try {
      await core.ops.op_read(rid, new Uint8Array(1));
      fail();
    } catch (e) {
      assertEquals(e.message, `Uh oh (${type})!`);
    }
    core.ops.op_close(rid);
  });
}

Deno.test(async function readableStreamEmptyOnStart() {
  const rid = resourceForReadableStream(emptyStream(true));
  const buffer = new Uint8Array(1024);
  const nread = await core.ops.op_read(rid, buffer);
  assertEquals(nread, 0);
  core.ops.op_close(rid);
});

Deno.test(async function readableStreamEmptyOnPull() {
  const rid = resourceForReadableStream(emptyStream(false));
  const buffer = new Uint8Array(1024);
  const nread = await core.ops.op_read(rid, buffer);
  assertEquals(nread, 0);
  core.ops.op_close(rid);
});

Deno.test(async function readableStreamEmptyReadAll() {
  const rid = resourceForReadableStream(emptyStream(false));
  const buffer = await core.ops.op_read_all(rid);
  assertEquals(buffer.length, 0);
  core.ops.op_close(rid);
});

Deno.test(async function readableStreamWithEmptyChunk() {
  const rid = resourceForReadableStream(emptyChunkStream());
  const buffer = await core.ops.op_read_all(rid);
  assertEquals(buffer, new Uint8Array([1, 2]));
  core.ops.op_close(rid);
});

Deno.test(async function readableStreamWithEmptyChunkOneByOne() {
  const rid = resourceForReadableStream(emptyChunkStream());
  assertEquals(1, await core.ops.op_read(rid, new Uint8Array(1)));
  assertEquals(1, await core.ops.op_read(rid, new Uint8Array(1)));
  assertEquals(0, await core.ops.op_read(rid, new Uint8Array(1)));
  core.ops.op_close(rid);
});

// Ensure that we correctly transmit all the sub-chunks of the larger chunks.
Deno.test(async function readableStreamReadSmallerChunks() {
  const packetSize = 16 * 1024;
  const rid = resourceForReadableStream(largePacketStream(packetSize, 1));
  const buffer = new Uint8Array(packetSize);
  for (let i = 0; i < packetSize / 1024; i++) {
    await core.ops.op_read(rid, buffer.subarray(i * 1024, i * 1024 + 1024));
  }
  for (let i = 0; i < 256; i++) {
    assertEquals(
      i,
      buffer[i * (packetSize / 256)],
      `at index ${i * (packetSize / 256)}`,
    );
  }
  core.ops.op_close(rid);
});

Deno.test(async function readableStreamLargePackets() {
  const packetSize = 128 * 1024;
  const rid = resourceForReadableStream(largePacketStream(packetSize, 1024));
  for (let i = 0; i < 1024; i++) {
    const buffer = new Uint8Array(packetSize);
    assertEquals(packetSize, await core.ops.op_read(rid, buffer));
    for (let i = 0; i < 256; i++) {
      assertEquals(
        i,
        buffer[i * (packetSize / 256)],
        `at index ${i * (packetSize / 256)}`,
      );
    }
  }
  assertEquals(0, await core.ops.op_read(rid, new Uint8Array(1)));
  core.ops.op_close(rid);
});

Deno.test(async function readableStreamVeryLargePackets() {
  // 1024 packets of 1MB
  const rid = resourceForReadableStream(largePacketStream(1024 * 1024, 1024));
  let total = 0;
  // Read 96kB up to 12,288 times (96kB is not an even multiple of the 1MB packet size to test this)
  const readCounts: Record<number, number> = {};
  for (let i = 0; i < 12 * 1024; i++) {
    const nread = await core.ops.op_read(rid, new Uint8Array(96 * 1024));
    total += nread;
    readCounts[nread] = (readCounts[nread] || 0) + 1;
    if (nread == 0) {
      break;
    }
  }
  assertEquals({ 0: 1, 65536: 1024, 98304: 10 * 1024 }, readCounts);
  assertEquals(total, 1024 * 1024 * 1024);
  core.ops.op_close(rid);
});

for (const count of [0, 1, 2, 3]) {
  for (const delay of [0, 1, 10]) {
    // Creating a stream that errors in start will throw
    if (delay > 0) {
      createStreamTest(count, delay, "Throw");
    }
    createStreamTest(count, delay, "Close");
  }
}

function createStreamTest(
  count: number,
  delay: number,
  action: "Throw" | "Close",
) {
  Deno.test(`streamCount${count}Delay${delay}${action}`, async () => {
    let rid;
    try {
      rid = resourceForReadableStream(
        makeStreamWithCount(count, delay, action),
      );
      for (let i = 0; i < count; i++) {
        const buffer = new Uint8Array(1);
        await core.ops.op_read(rid, buffer);
      }
      if (action == "Throw") {
        try {
          const buffer = new Uint8Array(1);
          assertEquals(1, await core.ops.op_read(rid, buffer));
          fail();
        } catch (e) {
          // We expect this to be thrown
          assertEquals(e.message, "Expected error!");
        }
      } else {
        const buffer = new Uint8Array(1);
        assertEquals(0, await core.ops.op_read(rid, buffer));
      }
    } finally {
      core.ops.op_close(rid);
    }
  });
}

Deno.test(async function readableStreamWithAggressiveResourceClose() {
  let first = true;
  const { promise: reasonPromise, resolve: reasonResolve } = Promise
    .withResolvers();
  const rid = resourceForReadableStream(
    new ReadableStream({
      pull(controller) {
        if (first) {
          // We queue this up and then immediately close the resource (not the reader)
          controller.enqueue(new Uint8Array(1));
          core.close(rid);
          // This doesn't throw, even though the resource is closed
          controller.enqueue(new Uint8Array(1));
          first = false;
        }
      },
      cancel(reason) {
        reasonResolve(reason);
      },
    }),
  );
  try {
    await core.ops.op_read(rid, new Uint8Array(1));
    fail();
  } catch (e) {
    assertEquals(e.message, "operation canceled");
  }
  assertEquals(await reasonPromise, "resource closed");
});
