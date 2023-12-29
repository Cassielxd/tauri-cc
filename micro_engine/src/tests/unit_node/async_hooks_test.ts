// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { AsyncLocalStorage, AsyncResource } from "node:async_hooks";
import {
  assert,
  assertEquals,
} from "../../../test_util/std/testing/asserts.ts";

Deno.test(async function foo() {
  const asyncLocalStorage = new AsyncLocalStorage();

  const out: string[] = [];
  function logWithId(msg: string) {
    const id = asyncLocalStorage.getStore();
    out.push(`${id !== undefined ? id : "-"}: ${msg}`);
  }

  async function exec() {
    logWithId("start");
    await new Promise((resolve) => setTimeout(resolve, 100));
    logWithId("finish");
  }

  for (const foo of [1, 2, 3]) {
    asyncLocalStorage.run(foo, exec);
  }

  await new Promise((resolve) => setTimeout(resolve, 500));

  assertEquals(out, [
    "1: start",
    "2: start",
    "3: start",
    "1: finish",
    "2: finish",
    "3: finish",
  ]);
});

Deno.test(async function bar() {
  let differentScopeDone = false;
  const als = new AsyncLocalStorage();
  const ac = new AbortController();
  const server = Deno.serve({
    signal: ac.signal,
    port: 4000,
  }, () => {
    const differentScope = als.run(123, () =>
      AsyncResource.bind(() => {
        differentScopeDone = true;
      }));
    return als.run("Hello World", async () => {
      // differentScope is attached to a different async context, so
      // it will see a different value for als.getStore() (123)
      setTimeout(differentScope, 5);
      // Some simulated async delay.
      await new Promise((res) => setTimeout(res, 10));
      return new Response(als.getStore() as string); // "Hello World"
    });
  });

  const res = await fetch("http://localhost:4000");
  assertEquals(await res.text(), "Hello World");
  ac.abort();
  await server.finished;
  assert(differentScopeDone);
});

Deno.test(async function nested() {
  const als = new AsyncLocalStorage();
  const deferred = Promise.withResolvers();
  const deferred1 = Promise.withResolvers();

  als.run(null, () => {
    als.run({ x: 1 }, () => {
      deferred.resolve(als.getStore());
    });
    deferred1.resolve(als.getStore());
  });

  assertEquals(await deferred.promise, { x: 1 });
  assertEquals(await deferred1.promise, null);
});
