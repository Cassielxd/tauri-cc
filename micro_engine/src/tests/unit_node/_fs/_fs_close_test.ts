// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import {
  assert,
  assertThrows,
  fail,
} from "../../../../test_util/std/testing/asserts.ts";
import { assertCallbackErrorUncaught } from "../_test_utils.ts";
import { close, closeSync } from "node:fs";

Deno.test({
  name: "ASYNC: File is closed",
  async fn() {
    const tempFile: string = await Deno.makeTempFile();
    const file: Deno.FsFile = await Deno.open(tempFile);

    assert(Deno.resources()[file.rid]);
    await new Promise<void>((resolve, reject) => {
      close(file.rid, (err) => {
        if (err !== null) reject();
        else resolve();
      });
    })
      .then(() => {
        assert(!Deno.resources()[file.rid]);
      }, () => {
        fail("No error expected");
      })
      .finally(async () => {
        await Deno.remove(tempFile);
      });
  },
});

Deno.test({
  name: "ASYNC: Invalid fd",
  fn() {
    assertThrows(() => {
      close(-1, (_err) => {});
    }, RangeError);
  },
});

Deno.test({
  name: "close callback should be asynchronous",
  async fn() {
    const tempFile: string = Deno.makeTempFileSync();
    const file: Deno.FsFile = Deno.openSync(tempFile);

    let foo: string;
    const promise = new Promise<void>((resolve) => {
      close(file.rid, () => {
        assert(foo === "bar");
        resolve();
      });
      foo = "bar";
    });

    await promise;
    Deno.removeSync(tempFile);
  },
});

Deno.test({
  name: "SYNC: File is closed",
  fn() {
    const tempFile: string = Deno.makeTempFileSync();
    const file: Deno.FsFile = Deno.openSync(tempFile);

    assert(Deno.resources()[file.rid]);
    closeSync(file.rid);
    assert(!Deno.resources()[file.rid]);
    Deno.removeSync(tempFile);
  },
});

Deno.test({
  name: "SYNC: Invalid fd",
  fn() {
    assertThrows(() => closeSync(-1));
  },
});

Deno.test("[std/node/fs] close callback isn't called twice if error is thrown", async () => {
  const tempFile = await Deno.makeTempFile();
  const importUrl = new URL("node:fs", import.meta.url);
  await assertCallbackErrorUncaught({
    prelude: `
    import { close } from ${JSON.stringify(importUrl)};

    const file = await Deno.open(${JSON.stringify(tempFile)});
    `,
    invocation: "close(file.rid, ",
    async cleanup() {
      await Deno.remove(tempFile);
    },
  });
});
