// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

import EventEmitter from "node:events";
import http, { type RequestOptions } from "node:http";
import https from "node:https";
import {
  assert,
  assertEquals,
  fail,
} from "../../../test_util/std/testing/asserts.ts";
import { assertSpyCalls, spy } from "../../../test_util/std/testing/mock.ts";

import { gzip } from "node:zlib";
import { Buffer } from "node:buffer";
import { serve } from "../../../test_util/std/http/server.ts";
import { execCode } from "../unit/test_util.ts";

Deno.test("[node/http listen]", async () => {
  {
    const server = http.createServer();
    assertEquals(0, EventEmitter.listenerCount(server, "request"));
  }

  {
    const server = http.createServer(() => {});
    assertEquals(1, EventEmitter.listenerCount(server, "request"));
  }

  {
    const { promise, resolve } = Promise.withResolvers<void>();
    const server = http.createServer();

    server.listen(() => {
      server.close();
    });
    server.on("close", () => {
      resolve();
    });

    await promise;
  }

  {
    const { promise, resolve } = Promise.withResolvers<void>();
    const server = http.createServer();

    server.listen().on("listening", () => {
      server.close();
    });
    server.on("close", () => {
      resolve();
    });

    await promise;
  }

  for (const port of [0, -0, 0.0, "0", null, undefined]) {
    const { promise, resolve } = Promise.withResolvers<void>();
    const server = http.createServer();

    server.listen(port, () => {
      server.close();
    });
    server.on("close", () => {
      resolve();
    });

    await promise;
  }
});

Deno.test("[node/http close]", async () => {
  {
    const deferred1 = Promise.withResolvers<void>();
    const deferred2 = Promise.withResolvers<void>();
    // Node quirk: callback gets exception object, event listener does not.
    // deno-lint-ignore no-explicit-any
    const server = http.createServer().close((err: any) => {
      assertEquals(err.code, "ERR_SERVER_NOT_RUNNING");
      deferred1.resolve();
    });
    // deno-lint-ignore no-explicit-any
    server.on("close", (err: any) => {
      assertEquals(err, undefined);
      deferred2.resolve();
    });
    server.on("listening", () => {
      throw Error("unreachable");
    });
    await deferred1.promise;
    await deferred2.promise;
  }

  {
    const deferred1 = Promise.withResolvers<void>();
    const deferred2 = Promise.withResolvers<void>();
    const server = http.createServer().listen().close((err) => {
      assertEquals(err, undefined);
      deferred1.resolve();
    });
    // deno-lint-ignore no-explicit-any
    server.on("close", (err: any) => {
      assertEquals(err, undefined);
      deferred2.resolve();
    });
    server.on("listening", () => {
      throw Error("unreachable");
    });
    await deferred1.promise;
    await deferred2.promise;
  }
});

Deno.test("[node/http] chunked response", async () => {
  for (
    const body of [undefined, "", "ok"]
  ) {
    const expected = body ?? "";
    const { promise, resolve } = Promise.withResolvers<void>();

    const server = http.createServer((_req, res) => {
      res.writeHead(200, { "transfer-encoding": "chunked" });
      res.end(body);
    });

    server.listen(async () => {
      const res = await fetch(
        // deno-lint-ignore no-explicit-any
        `http://127.0.0.1:${(server.address() as any).port}/`,
      );
      assert(res.ok);

      const actual = await res.text();
      assertEquals(actual, expected);

      server.close(() => resolve());
    });

    await promise;
  }
});

// Test empty chunks: https://github.com/denoland/deno/issues/17194
Deno.test("[node/http] empty chunk in the middle of response", async () => {
  const { promise, resolve } = Promise.withResolvers<void>();

  const server = http.createServer((_req, res) => {
    res.write("a");
    res.write("");
    res.write("b");
    res.end();
  });

  server.listen(async () => {
    const res = await fetch(
      // deno-lint-ignore no-explicit-any
      `http://127.0.0.1:${(server.address() as any).port}/`,
    );
    const actual = await res.text();
    assertEquals(actual, "ab");
    server.close(() => resolve());
  });

  await promise;
});

Deno.test("[node/http] server can respond with 101, 204, 205, 304 status", async () => {
  for (const status of [101, 204, 205, 304]) {
    const { promise, resolve } = Promise.withResolvers<void>();
    const server = http.createServer((_req, res) => {
      res.statusCode = status;
      res.end("");
    });
    server.listen(async () => {
      const res = await fetch(
        // deno-lint-ignore no-explicit-any
        `http://127.0.0.1:${(server.address() as any).port}/`,
      );
      await res.arrayBuffer();
      assertEquals(res.status, status);
      server.close(() => resolve());
    });
    await promise;
  }
});

Deno.test("[node/http] request default protocol", async () => {
  const deferred1 = Promise.withResolvers<void>();
  const deferred2 = Promise.withResolvers<void>();
  const server = http.createServer((_, res) => {
    res.end("ok");
  });

  // @ts-ignore IncomingMessageForClient
  // deno-lint-ignore no-explicit-any
  let clientRes: any;
  // deno-lint-ignore no-explicit-any
  let clientReq: any;
  server.listen(() => {
    clientReq = http.request(
      // deno-lint-ignore no-explicit-any
      { host: "localhost", port: (server.address() as any).port },
      (res) => {
        assert(res.socket instanceof EventEmitter);
        assertEquals(res.complete, false);
        res.on("data", () => {});
        res.on("end", () => {
          server.close();
        });
        clientRes = res;
        assertEquals(res.statusCode, 200);
        deferred2.resolve();
      },
    );
    clientReq.end();
  });
  server.on("close", () => {
    deferred1.resolve();
  });
  await deferred1.promise;
  await deferred2.promise;
  assert(clientReq.socket instanceof EventEmitter);
  assertEquals(clientRes!.complete, true);
});

Deno.test("[node/http] request with headers", async () => {
  const { promise, resolve } = Promise.withResolvers<void>();
  const server = http.createServer((req, res) => {
    assertEquals(req.headers["x-foo"], "bar");
    res.end("ok");
  });
  server.listen(() => {
    const req = http.request(
      {
        host: "localhost",
        // deno-lint-ignore no-explicit-any
        port: (server.address() as any).port,
        headers: { "x-foo": "bar" },
      },
      (res) => {
        res.on("data", () => {});
        res.on("end", () => {
          server.close();
        });
        assertEquals(res.statusCode, 200);
      },
    );
    req.end();
  });
  server.on("close", () => {
    resolve();
  });
  await promise;
});

Deno.test("[node/http] non-string buffer response", {
  // TODO(kt3k): Enable sanitizer. A "zlib" resource is leaked in this test case.
  sanitizeResources: false,
}, async () => {
  const { promise, resolve } = Promise.withResolvers<void>();
  const server = http.createServer((_, res) => {
    res.socket!.end();
    gzip(
      Buffer.from("a".repeat(100), "utf8"),
      {},
      (_err: Error | null, data: Buffer) => {
        res.setHeader("Content-Encoding", "gzip");
        res.end(data);
      },
    );
  });
  server.listen(async () => {
    const res = await fetch(
      // deno-lint-ignore no-explicit-any
      `http://localhost:${(server.address() as any).port}`,
    );
    try {
      const text = await res.text();
      assertEquals(text, "a".repeat(100));
    } catch (e) {
      server.emit("error", e);
    } finally {
      server.close(() => resolve());
    }
  });
  await promise;
});

// TODO(kt3k): Enable this test
// Currently IncomingMessage constructor has incompatible signature.
/*
Deno.test("[node/http] http.IncomingMessage can be created without url", () => {
  const message = new http.IncomingMessage(
    // adapted from https://github.com/dougmoscrop/serverless-http/blob/80bfb3e940057d694874a8b0bc12ad96d2abe7ab/lib/request.js#L7
    {
      // @ts-expect-error - non-request properties will also be passed in, e.g. by serverless-http
      encrypted: true,
      readable: false,
      remoteAddress: "foo",
      address: () => ({ port: 443 }),
      // deno-lint-ignore no-explicit-any
      end: Function.prototype as any,
      // deno-lint-ignore no-explicit-any
      destroy: Function.prototype as any,
    },
  );
  message.url = "https://example.com";
});
*/

Deno.test("[node/http] send request with non-chunked body", async () => {
  let requestHeaders: Headers;
  let requestBody = "";

  const hostname = "localhost";
  const port = 4505;

  // NOTE: Instead of node/http.createServer(), serve() in std/http/server.ts is used.
  // https://github.com/denoland/deno_std/pull/2755#discussion_r1005592634
  const handler = async (req: Request) => {
    requestHeaders = req.headers;
    requestBody = await req.text();
    return new Response("ok");
  };
  const abortController = new AbortController();
  const servePromise = serve(handler, {
    hostname,
    port,
    signal: abortController.signal,
    onListen: undefined,
  });

  const opts: RequestOptions = {
    host: hostname,
    port,
    method: "POST",
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Content-Length": "11",
    },
  };
  const req = http.request(opts, (res) => {
    res.on("data", () => {});
    res.on("end", () => {
      abortController.abort();
    });
    assertEquals(res.statusCode, 200);
    assertEquals(requestHeaders.get("content-length"), "11");
    assertEquals(requestHeaders.has("transfer-encoding"), false);
    assertEquals(requestBody, "hello world");
  });
  req.on("socket", (socket) => {
    assert(socket.writable);
    assert(socket.readable);
    socket.setKeepAlive();
    socket.destroy();
    socket.setTimeout(100);
  });
  req.write("hello ");
  req.write("world");
  req.end();

  await servePromise;
});

Deno.test("[node/http] send request with chunked body", async () => {
  let requestHeaders: Headers;
  let requestBody = "";

  const hostname = "localhost";
  const port = 4505;

  // NOTE: Instead of node/http.createServer(), serve() in std/http/server.ts is used.
  // https://github.com/denoland/deno_std/pull/2755#discussion_r1005592634
  const handler = async (req: Request) => {
    requestHeaders = req.headers;
    requestBody = await req.text();
    return new Response("ok");
  };
  const abortController = new AbortController();
  const servePromise = serve(handler, {
    hostname,
    port,
    signal: abortController.signal,
    onListen: undefined,
  });

  const opts: RequestOptions = {
    host: hostname,
    port,
    method: "POST",
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Content-Length": "11",
      "Transfer-Encoding": "chunked",
    },
  };
  const req = http.request(opts, (res) => {
    res.on("data", () => {});
    res.on("end", () => {
      abortController.abort();
    });
    assertEquals(res.statusCode, 200);
    assertEquals(requestHeaders.has("content-length"), false);
    assertEquals(requestHeaders.get("transfer-encoding"), "chunked");
    assertEquals(requestBody, "hello world");
  });
  req.write("hello ");
  req.write("world");
  req.end();

  await servePromise;
});

Deno.test("[node/http] send request with chunked body as default", async () => {
  let requestHeaders: Headers;
  let requestBody = "";

  const hostname = "localhost";
  const port = 4505;

  // NOTE: Instead of node/http.createServer(), serve() in std/http/server.ts is used.
  // https://github.com/denoland/deno_std/pull/2755#discussion_r1005592634
  const handler = async (req: Request) => {
    requestHeaders = req.headers;
    requestBody = await req.text();
    return new Response("ok");
  };
  const abortController = new AbortController();
  const servePromise = serve(handler, {
    hostname,
    port,
    signal: abortController.signal,
    onListen: undefined,
  });

  const opts: RequestOptions = {
    host: hostname,
    port,
    method: "POST",
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
    },
  };
  const req = http.request(opts, (res) => {
    res.on("data", () => {});
    res.on("end", () => {
      abortController.abort();
    });
    assertEquals(res.statusCode, 200);
    assertEquals(requestHeaders.has("content-length"), false);
    assertEquals(requestHeaders.get("transfer-encoding"), "chunked");
    assertEquals(requestBody, "hello world");
  });
  req.write("hello ");
  req.write("world");
  req.end();

  await servePromise;
});

Deno.test("[node/http] ServerResponse _implicitHeader", async () => {
  const { promise, resolve } = Promise.withResolvers<void>();
  const server = http.createServer((_req, res) => {
    const writeHeadSpy = spy(res, "writeHead");
    // deno-lint-ignore no-explicit-any
    (res as any)._implicitHeader();
    assertSpyCalls(writeHeadSpy, 1);
    writeHeadSpy.restore();
    res.end("Hello World");
  });

  server.listen(async () => {
    const { port } = server.address() as { port: number };
    const res = await fetch(`http://localhost:${port}`);
    assertEquals(await res.text(), "Hello World");
    server.close(() => {
      resolve();
    });
  });

  await promise;
});

Deno.test("[node/http] server unref", async () => {
  const [statusCode, _output] = await execCode(`
  import http from "node:http";
  const server = http.createServer((_req, res) => {
    res.statusCode = status;
    res.end("");
  });

  // This should let the program to exit without waiting for the
  // server to close.
  server.unref();

  server.listen(async () => {
  });
  `);
  assertEquals(statusCode, 0);
});

Deno.test("[node/http] ClientRequest handle non-string headers", async () => {
  // deno-lint-ignore no-explicit-any
  let headers: any;
  const { promise, resolve, reject } = Promise.withResolvers<void>();
  const req = http.request("http://localhost:4545/echo_server", {
    method: "POST",
    headers: { 1: 2 },
  }, (resp) => {
    headers = resp.headers;

    resp.on("data", () => {});

    resp.on("end", () => {
      resolve();
    });
  });
  req.once("error", (e) => reject(e));
  req.end();
  await promise;
  assertEquals(headers!["1"], "2");
});

Deno.test("[node/http] ClientRequest uses HTTP/1.1", async () => {
  let body = "";
  const { promise, resolve, reject } = Promise.withResolvers<void>();
  const req = https.request("https://localhost:5545/http_version", {
    method: "POST",
    headers: { 1: 2 },
  }, (resp) => {
    resp.on("data", (chunk) => {
      body += chunk;
    });

    resp.on("end", () => {
      resolve();
    });
  });
  req.once("error", (e) => reject(e));
  req.end();
  await promise;
  assertEquals(body, "HTTP/1.1");
});

Deno.test("[node/http] ClientRequest setTimeout", async () => {
  let body = "";
  const { promise, resolve, reject } = Promise.withResolvers<void>();
  const timer = setTimeout(() => reject("timed out"), 50000);
  const req = http.request("http://localhost:4545/http_version", (resp) => {
    resp.on("data", (chunk) => {
      body += chunk;
    });

    resp.on("end", () => {
      resolve();
    });
  });
  req.setTimeout(120000);
  req.once("error", (e) => reject(e));
  req.end();
  await promise;
  clearTimeout(timer);
  assertEquals(body, "HTTP/1.1");
});

Deno.test("[node/http] ClientRequest PATCH", async () => {
  let body = "";
  const { promise, resolve, reject } = Promise.withResolvers<void>();
  const req = http.request("http://localhost:4545/echo_server", {
    method: "PATCH",
  }, (resp) => {
    resp.on("data", (chunk) => {
      body += chunk;
    });

    resp.on("end", () => {
      resolve();
    });
  });
  req.write("hello ");
  req.write("world");
  req.once("error", (e) => reject(e));
  req.end();
  await promise;
  assertEquals(body, "hello world");
});

Deno.test("[node/http] ClientRequest PUT", async () => {
  let body = "";
  const { promise, resolve, reject } = Promise.withResolvers<void>();
  const req = http.request("http://localhost:4545/echo_server", {
    method: "PUT",
  }, (resp) => {
    resp.on("data", (chunk) => {
      body += chunk;
    });

    resp.on("end", () => {
      resolve();
    });
  });
  req.write("hello ");
  req.write("world");
  req.once("error", (e) => reject(e));
  req.end();
  await promise;
  assertEquals(body, "hello world");
});

Deno.test("[node/http] ClientRequest search params", async () => {
  let body = "";
  const { promise, resolve, reject } = Promise.withResolvers<void>();
  const req = http.request({
    host: "localhost:4545",
    path: "search_params?foo=bar",
  }, (resp) => {
    resp.on("data", (chunk) => {
      body += chunk;
    });

    resp.on("end", () => {
      resolve();
    });
  });
  req.once("error", (e) => reject(e));
  req.end();
  await promise;
  assertEquals(body, "foo=bar");
});

Deno.test("[node/http] HTTPS server", async () => {
  const deferred = Promise.withResolvers<void>();
  const deferred2 = Promise.withResolvers<void>();
  const client = Deno.createHttpClient({
    caCerts: [Deno.readTextFileSync("cli/tests/testdata/tls/RootCA.pem")],
  });
  const server = https.createServer({
    cert: Deno.readTextFileSync("cli/tests/testdata/tls/localhost.crt"),
    key: Deno.readTextFileSync("cli/tests/testdata/tls/localhost.key"),
  }, (req, res) => {
    // @ts-ignore: It exists on TLSSocket
    assert(req.socket.encrypted);
    res.end("success!");
  });
  server.listen(() => {
    // deno-lint-ignore no-explicit-any
    fetch(`https://localhost:${(server.address() as any).port}`, {
      client,
    }).then(async (res) => {
      assertEquals(res.status, 200);
      assertEquals(await res.text(), "success!");
      server.close();
      deferred2.resolve();
    });
  })
    .on("error", () => fail());
  server.on("close", () => {
    deferred.resolve();
  });
  await Promise.all([deferred.promise, deferred2.promise]);
  client.close();
});

Deno.test(
  "[node/http] client upgrade",
  { permissions: { net: true } },
  async () => {
    const { promise, resolve } = Promise.withResolvers<void>();
    const server = http.createServer((req, res) => {
      // @ts-ignore: It exists on TLSSocket
      assert(!req.socket.encrypted);
      res.writeHead(200, { "Content-Type": "text/plain" });
      res.end("okay");
    });
    // @ts-ignore it's a socket for real
    let serverSocket;
    server.on("upgrade", (_req, socket, _head) => {
      socket.write(
        "HTTP/1.1 101 Web Socket Protocol Handshake\r\n" +
          "Upgrade: WebSocket\r\n" +
          "Connection: Upgrade\r\n" +
          "\r\n",
      );
      serverSocket = socket;
    });

    // Now that server is running
    server.listen(1337, "127.0.0.1", () => {
      // make a request
      const options = {
        port: 1337,
        host: "127.0.0.1",
        headers: {
          "Connection": "Upgrade",
          "Upgrade": "websocket",
        },
      };

      const req = http.request(options);
      req.end();

      req.on("upgrade", (_res, socket, _upgradeHead) => {
        socket.end();
        // @ts-ignore it's a socket for real
        serverSocket!.end();
        server.close(() => {
          resolve();
        });
      });
    });

    await promise;
  },
);

Deno.test(
  "[node/http] client end with callback",
  { permissions: { net: true } },
  async () => {
    let received = false;
    const ac = new AbortController();
    const server = Deno.serve({ port: 5928, signal: ac.signal }, (_req) => {
      received = true;
      return new Response("hello");
    });
    const { promise, resolve, reject } = Promise.withResolvers<void>();
    let body = "";

    const request = http.request(
      "http://localhost:5928/",
      (resp) => {
        resp.on("data", (chunk) => {
          body += chunk;
        });

        resp.on("end", () => {
          resolve();
        });
      },
    );
    request.on("error", reject);
    request.end(() => {
      assert(received);
    });

    await promise;
    ac.abort();
    await server.finished;

    assertEquals(body, "hello");
  },
);

Deno.test("[node/http] server emits error if addr in use", async () => {
  const deferred1 = Promise.withResolvers<void>();
  const deferred2 = Promise.withResolvers<Error>();

  const server = http.createServer();
  server.listen(9001);

  const server2 = http.createServer();
  server2.on("error", (e) => {
    deferred2.resolve(e);
  });
  server2.listen(9001);

  const err = await deferred2.promise;
  server.close(() => deferred1.resolve());
  server2.close();
  await deferred1.promise;
  const expectedMsg = Deno.build.os === "windows"
    ? "Only one usage of each socket address"
    : "Address already in use";
  assert(
    err.message.startsWith(expectedMsg),
    `Wrong error: ${err.message}`,
  );
});

Deno.test(
  "[node/http] client destroy doesn't leak",
  { permissions: { net: true } },
  async () => {
    const ac = new AbortController();
    let timerId;

    const server = Deno.serve(
      { port: 5929, signal: ac.signal },
      async (_req) => {
        await new Promise((resolve) => {
          timerId = setTimeout(resolve, 5000);
        });
        return new Response("hello");
      },
    );
    const { promise, resolve, reject } = Promise.withResolvers<void>();

    const request = http.request("http://localhost:5929/");
    request.on("error", reject);
    request.on("close", () => {});
    request.end();
    setTimeout(() => {
      request.destroy(new Error());
      resolve();
    }, 100);

    await promise;
    clearTimeout(timerId);
    ac.abort();
    await server.finished;
  },
);

Deno.test("[node/http] node:http exports globalAgent", async () => {
  const http = await import("node:http");
  assert(
    http.globalAgent,
    "node:http must export 'globalAgent' on module namespace",
  );
  assert(
    http.default.globalAgent,
    "node:http must export 'globalAgent' on module default export",
  );
});

Deno.test("[node/https] node:https exports globalAgent", async () => {
  const https = await import("node:https");
  assert(
    https.globalAgent,
    "node:https must export 'globalAgent' on module namespace",
  );
  assert(
    https.default.globalAgent,
    "node:https must export 'globalAgent' on module default export",
  );
});
