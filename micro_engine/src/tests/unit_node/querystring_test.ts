// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { assertEquals } from "../../../test_util/std/testing/asserts.ts";
import { parse, stringify } from "node:querystring";

Deno.test({
  name: "stringify",
  fn() {
    assertEquals(
      stringify({
        a: "hello",
        b: 5,
        c: true,
        d: ["foo", "bar"],
      }),
      "a=hello&b=5&c=true&d=foo&d=bar",
    );
  },
});

Deno.test({
  name: "parse",
  fn() {
    assertEquals(parse("a=hello&b=5&c=true&d=foo&d=bar"), {
      a: "hello",
      b: "5",
      c: "true",
      d: ["foo", "bar"],
    });
  },
});
