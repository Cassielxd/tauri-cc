// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use test_util as util;

#[test]
fn eval_p() {
  let output = util::deno_cmd()
    .arg("eval")
    .arg("-p")
    .arg("1+2")
    .stdout_piped()
    .spawn()
    .unwrap()
    .wait_with_output()
    .unwrap();
  assert!(output.status.success());
  let stdout_str =
    util::strip_ansi_codes(std::str::from_utf8(&output.stdout).unwrap().trim());
  assert_eq!("3", stdout_str);
}

// Make sure that snapshot flags don't affect runtime.
#[test]
fn eval_randomness() {
  let mut numbers = Vec::with_capacity(10);
  for _ in 0..10 {
    let output = util::deno_cmd()
      .arg("eval")
      .arg("-p")
      .arg("Math.random()")
      .stdout_piped()
      .spawn()
      .unwrap()
      .wait_with_output()
      .unwrap();
    assert!(output.status.success());
    let stdout_str = util::strip_ansi_codes(
      std::str::from_utf8(&output.stdout).unwrap().trim(),
    );
    numbers.push(stdout_str.to_string());
  }
  numbers.dedup();
  assert!(numbers.len() > 1);
}

itest!(eval_basic {
  args: "eval console.log(\"hello\")",
  output_str: Some("hello\n"),
});

// Ugly parentheses due to whitespace delimiting problem.
itest!(eval_ts {
  args: "eval --quiet --ext=ts console.log((123)as(number))", // 'as' is a TS keyword only
  output_str: Some("123\n"),
});

itest!(dyn_import_eval {
  args: "eval import('./subdir/mod4.js').then(console.log)",
  output: "eval/dyn_import_eval.out",
});

// Cannot write the expression to evaluate as "console.log(typeof gc)"
// because itest! splits args on whitespace.
itest!(v8_flags_eval {
  args: "eval --v8-flags=--expose-gc console.log(typeof(gc))",
  output: "run/v8_flags.js.out",
});

itest!(check_local_by_default {
  args: "eval --quiet import('http://localhost:4545/subdir/type_error.ts').then(console.log);",
  output: "eval/check_local_by_default.out",
  http_server: true,
});

itest!(check_local_by_default2 {
  args: "eval --quiet import('./eval/check_local_by_default2.ts').then(console.log);",
  output: "eval/check_local_by_default2.out",
  http_server: true,
});

itest!(env_file {
  args: "eval --env=env console.log(Deno.env.get(\"ANOTHER_FOO\"))",
  output_str: Some("ANOTHER_BAR\n"),
});

itest!(env_file_missing {
  args: "eval --env=missing console.log(Deno.env.get(\"ANOTHER_FOO\"))",
  output_str: Some(
    "error: Unable to load 'missing' environment variable file\n"
  ),
  exit_code: 1,
});
