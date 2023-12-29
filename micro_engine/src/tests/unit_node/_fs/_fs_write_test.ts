// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { write, writeSync } from "node:fs";
import { assertEquals } from "../../../../test_util/std/testing/asserts.ts";
import { Buffer } from "node:buffer";

const decoder = new TextDecoder("utf-8");

Deno.test({
  name: "Data is written to the file with the correct length",
  async fn() {
    const tempFile: string = await Deno.makeTempFile();
    const file: Deno.FsFile = await Deno.open(tempFile, {
      create: true,
      write: true,
      read: true,
    });
    const buffer = Buffer.from("hello world");
    const bytesWrite = await new Promise((resolve, reject) => {
      write(file.rid, buffer, 0, 5, (err: unknown, nwritten: number) => {
        if (err) return reject(err);
        resolve(nwritten);
      });
    });
    Deno.close(file.rid);

    const data = await Deno.readFile(tempFile);
    await Deno.remove(tempFile);

    assertEquals(bytesWrite, 5);
    assertEquals(decoder.decode(data), "hello");
  },
});

Deno.test({
  name: "Data is written synchronously to the file with the correct length",
  fn() {
    const tempFile: string = Deno.makeTempFileSync();
    const file: Deno.FsFile = Deno.openSync(tempFile, {
      create: true,
      write: true,
      read: true,
    });
    const buffer = Buffer.from("hello world");
    const bytesWrite = writeSync(file.rid, buffer, 0, 5);
    Deno.close(file.rid);

    const data = Deno.readFileSync(tempFile);
    Deno.removeSync(tempFile);

    assertEquals(bytesWrite, 5);
    assertEquals(decoder.decode(data), "hello");
  },
});
