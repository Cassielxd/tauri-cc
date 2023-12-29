// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use test_util as util;
use util::assert_contains;
use util::assert_not_contains;
use util::testdata_path;
use util::TestContext;
use util::TestContextBuilder;

#[test]
fn compile_basic() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("welcome.exe")
  } else {
    dir.path().join("welcome")
  };
  // try this twice to ensure it works with the cache
  for _ in 0..2 {
    let output = context
      .new_command()
      .args_vec([
        "compile",
        "--output",
        &exe.to_string_lossy(),
        "../../../test_util/std/examples/welcome.ts",
      ])
      .run();
    output.assert_exit_code(0);
    output.skip_output_check();
    let output = context.new_command().name(&exe).run();
    output.assert_matches_text("Welcome to Deno!\n");
  }

  // now ensure this works when the deno_dir is readonly
  let readonly_dir = dir.path().join("readonly");
  readonly_dir.make_dir_readonly();
  let readonly_sub_dir = readonly_dir.join("sub");

  let output = context
    .new_command()
    // it should fail creating this, but still work
    .env("DENO_DIR", readonly_sub_dir)
    .name(exe)
    .run();
  output.assert_matches_text("Welcome to Deno!\n");
}

#[test]
fn standalone_args() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("args.exe")
  } else {
    dir.path().join("args")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./compile/args.ts",
      "a",
      "b",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);
  context
    .new_command()
    .name(&exe)
    .args("foo --bar --unstable")
    .run()
    .assert_matches_text("a\nb\nfoo\n--bar\n--unstable\n")
    .assert_exit_code(0);
}

#[test]
fn standalone_error() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("error.exe")
  } else {
    dir.path().join("error")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./compile/standalone_error.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);

  let output = context.new_command().name(&exe).split_output().run();
  output.assert_exit_code(1);
  output.assert_stdout_matches_text("");
  let stderr = output.stderr();
  // On Windows, we cannot assert the file path (because '\').
  // Instead we just check for relevant output.
  assert_contains!(stderr, "error: Uncaught Error: boom!");
  assert_contains!(stderr, "throw new Error(\"boom!\");");
  assert_contains!(stderr, "\n    at boom (file://");
  assert_contains!(stderr, "standalone_error.ts:2:9");
  assert_contains!(stderr, "at foo (file://");
  assert_contains!(stderr, "standalone_error.ts:5:3");
  assert_contains!(stderr, "standalone_error.ts:7:1");
}

#[test]
fn standalone_error_module_with_imports() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("error.exe")
  } else {
    dir.path().join("error")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./compile/standalone_error_module_with_imports_1.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);

  let output = context
    .new_command()
    .name(&exe)
    .env("NO_COLOR", "1")
    .split_output()
    .run();
  output.assert_stdout_matches_text("hello\n");
  let stderr = output.stderr();
  // On Windows, we cannot assert the file path (because '\').
  // Instead we just check for relevant output.
  assert_contains!(stderr, "error: Uncaught Error: boom!");
  assert_contains!(stderr, "throw new Error(\"boom!\");");
  assert_contains!(stderr, "\n    at file://");
  assert_contains!(stderr, "standalone_error_module_with_imports_2.ts:2:7");
  output.assert_exit_code(1);
}

#[test]
fn standalone_load_datauri() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("load_datauri.exe")
  } else {
    dir.path().join("load_datauri")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./compile/standalone_import_datauri.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);
  context
    .new_command()
    .name(&exe)
    .run()
    .assert_matches_text("Hello Deno!\n")
    .assert_exit_code(0);
}

// https://github.com/denoland/deno/issues/13704
#[test]
fn standalone_follow_redirects() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("follow_redirects.exe")
  } else {
    dir.path().join("follow_redirects")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./compile/standalone_follow_redirects.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);
  context
    .new_command()
    .name(&exe)
    .run()
    .assert_matches_text("Hello\n")
    .assert_exit_code(0);
}

#[test]
fn compile_with_file_exists_error() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let output_path = if cfg!(windows) {
    dir.path().join(r"args\")
  } else {
    dir.path().join("args/")
  };
  let file_path = dir.path().join("args");
  file_path.write("");
  context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &output_path.to_string_lossy(),
      "./compile/args.ts",
    ])
    .run()
    .assert_matches_text(&format!(
      concat!(
        "[WILDCARD]error: Could not compile to file '{}' because its parent directory ",
        "is an existing file. You can use the `--output <file-path>` flag to ",
        "provide an alternative name.\n",
      ),
      file_path,
    ))
    .assert_exit_code(1);
}

#[test]
fn compile_with_directory_exists_error() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("args.exe")
  } else {
    dir.path().join("args")
  };
  std::fs::create_dir(&exe).unwrap();
  context.new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./compile/args.ts"
    ]).run()
    .assert_matches_text(&format!(
      concat!(
        "[WILDCARD]error: Could not compile to file '{}' because a directory exists with ",
        "the same name. You can use the `--output <file-path>` flag to ",
        "provide an alternative name.\n"
      ),
      exe
    ))
    .assert_exit_code(1);
}

#[test]
fn compile_with_conflict_file_exists_error() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("args.exe")
  } else {
    dir.path().join("args")
  };
  std::fs::write(&exe, b"SHOULD NOT BE OVERWRITTEN").unwrap();
  context.new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./compile/args.ts"
    ]).run()
    .assert_matches_text(&format!(
      concat!(
        "[WILDCARD]error: Could not compile to file '{}' because the file already exists ",
        "and cannot be overwritten. Please delete the existing file or ",
        "use the `--output <file-path>` flag to provide an alternative name.\n"
      ),
      exe
    ))
    .assert_exit_code(1);
  exe.assert_matches_text("SHOULD NOT BE OVERWRITTEN");
}

#[test]
fn compile_and_overwrite_file() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("args.exe")
  } else {
    dir.path().join("args")
  };

  // do this twice
  for _ in 0..2 {
    context
      .new_command()
      .args_vec([
        "compile",
        "--output",
        &exe.to_string_lossy(),
        "./compile/args.ts",
      ])
      .run()
      .skip_output_check()
      .assert_exit_code(0);
    assert!(&exe.exists());
  }
}

#[test]
fn standalone_runtime_flags() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("flags.exe")
  } else {
    dir.path().join("flags")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--allow-read",
      "--seed",
      "1",
      "--output",
      &exe.to_string_lossy(),
      "./compile/standalone_runtime_flags.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);
  context
    .new_command()
    .env("NO_COLOR", "1")
    .name(&exe)
    .split_output()
    .run()
    .assert_stdout_matches_text("0.147205063401058\n")
    .assert_stderr_matches_text(
      "[WILDCARD]PermissionDenied: Requires write access to[WILDCARD]",
    )
    .assert_exit_code(1);
}

#[test]
fn standalone_ext_flag_ts() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("ext_flag_ts.exe")
  } else {
    dir.path().join("ext_flag_ts")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--ext",
      "ts",
      "--output",
      &exe.to_string_lossy(),
      "./file_extensions/ts_without_extension",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);
  context
    .new_command()
    .env("NO_COLOR", "1")
    .name(&exe)
    .run()
    .assert_matches_text("executing typescript with no extension\n")
    .assert_exit_code(0);
}

#[test]
fn standalone_ext_flag_js() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("ext_flag_js.exe")
  } else {
    dir.path().join("ext_flag_js")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--ext",
      "js",
      "--output",
      &exe.to_string_lossy(),
      "./file_extensions/js_without_extension",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);
  context
    .new_command()
    .env("NO_COLOR", "1")
    .name(&exe)
    .run()
    .assert_matches_text("executing javascript with no extension\n");
}

#[test]
fn standalone_import_map() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("import_map.exe")
  } else {
    dir.path().join("import_map")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--allow-read",
      "--import-map",
      "compile/standalone_import_map.json",
      "--output",
      &exe.to_string_lossy(),
      "./compile/standalone_import_map.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);
  context
    .new_command()
    .name(&exe)
    .run()
    .skip_output_check()
    .assert_exit_code(0);
}

#[test]
fn standalone_import_map_config_file() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("import_map.exe")
  } else {
    dir.path().join("import_map")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--allow-read",
      "--config",
      "compile/standalone_import_map_config.json",
      "--output",
      &exe.to_string_lossy(),
      "./compile/standalone_import_map.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);
  context
    .new_command()
    .name(&exe)
    .run()
    .skip_output_check()
    .assert_exit_code(0);
}

#[test]
// https://github.com/denoland/deno/issues/12670
fn skip_rebundle() {
  let context = TestContextBuilder::new().build();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("hello_world.exe")
  } else {
    dir.path().join("hello_world")
  };
  let output = context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./run/001_hello.js",
    ])
    .run();

  //no "Bundle testdata_path/run/001_hello.js" in output
  assert_not_contains!(output.combined_output(), "Bundle");

  context
    .new_command()
    .name(&exe)
    .run()
    .assert_matches_text("Hello World\n")
    .assert_exit_code(0);
}

#[test]
fn check_local_by_default() {
  let context = TestContext::with_http_server();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("welcome.exe")
  } else {
    dir.path().join("welcome")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./compile/check_local_by_default.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);
}

#[test]
fn check_local_by_default2() {
  let context = TestContext::with_http_server();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("welcome.exe")
  } else {
    dir.path().join("welcome")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./compile/check_local_by_default2.ts"
    ])
    .run()
    .assert_matches_text(
      r#"[WILDCARD]error: TS2322 [ERROR]: Type '12' is not assignable to type '"b"'.[WILDCARD]"#,
    )
    .assert_exit_code(1);
}

#[test]
fn workers_basic() {
  let context = TestContext::with_http_server();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("basic.exe")
  } else {
    dir.path().join("basic")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--no-check",
      "--output",
      &exe.to_string_lossy(),
      "./compile/workers/basic.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);

  context
    .new_command()
    .name(&exe)
    .run()
    .assert_matches_file("./compile/workers/basic.out")
    .assert_exit_code(0);
}

#[test]
fn workers_not_in_module_map() {
  let context = TestContext::with_http_server();
  let temp_dir = context.temp_dir();
  let exe = if cfg!(windows) {
    temp_dir.path().join("not_in_module_map.exe")
  } else {
    temp_dir.path().join("not_in_module_map")
  };
  let output = context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./compile/workers/not_in_module_map.ts",
    ])
    .run();
  output.assert_exit_code(0);
  output.skip_output_check();

  let output = context.new_command().name(exe).env("NO_COLOR", "").run();
  output.assert_exit_code(1);
  output.assert_matches_text(concat!(
    "error: Uncaught (in worker \"\") Module not found: [WILDCARD]",
    "error: Uncaught (in promise) Error: Unhandled error in child worker.\n[WILDCARD]"
  ));
}

#[test]
fn workers_with_include_flag() {
  let context = TestContext::with_http_server();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("workers_with_include_flag.exe")
  } else {
    dir.path().join("workers_with_include_flag")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "--include",
      "./compile/workers/worker.ts",
      "./compile/workers/not_in_module_map.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);

  context
    .new_command()
    .name(&exe)
    .env("NO_COLOR", "")
    .run()
    .assert_matches_text("Hello from worker!\nReceived 42\nClosing\n");
}

#[test]
fn dynamic_import() {
  let context = TestContext::with_http_server();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("dynamic_import.exe")
  } else {
    dir.path().join("dynamic_import")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--output",
      &exe.to_string_lossy(),
      "./compile/dynamic_imports/main.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);

  context
    .new_command()
    .name(&exe)
    .env("NO_COLOR", "")
    .run()
    .assert_matches_file("./compile/dynamic_imports/main.out")
    .assert_exit_code(0);
}

#[test]
fn dynamic_import_unanalyzable() {
  let context = TestContext::with_http_server();
  let dir = context.temp_dir();
  let exe = if cfg!(windows) {
    dir.path().join("dynamic_import_unanalyzable.exe")
  } else {
    dir.path().join("dynamic_import_unanalyzable")
  };
  context
    .new_command()
    .args_vec([
      "compile",
      "--allow-read",
      "--include",
      "./compile/dynamic_imports/import1.ts",
      "--output",
      &exe.to_string_lossy(),
      "./compile/dynamic_imports/main_unanalyzable.ts",
    ])
    .run()
    .skip_output_check()
    .assert_exit_code(0);

  context
    .new_command()
    .current_dir(util::root_path().join("cli"))
    .name(&exe)
    .env("NO_COLOR", "")
    .run()
    .assert_matches_file("./compile/dynamic_imports/main.out")
    .assert_exit_code(0);
}

#[test]
fn compile_npm_specifiers() {
  let context = TestContextBuilder::for_npm().use_temp_cwd().build();

  let temp_dir = context.temp_dir();
  temp_dir.write(
    "main.ts",
    concat!(
      "import path from 'node:path';\n",
      "import { getValue, setValue } from 'npm:@denotest/esm-basic';\n",
      "import getValueDefault from 'npm:@denotest/esm-import-cjs-default';\n",
      "setValue(2);\n",
      "console.log(path.join('testing', 'this'));",
      "console.log(getValue());",
      "console.log(getValueDefault());",
    ),
  );

  let binary_path = if cfg!(windows) {
    temp_dir.path().join("binary.exe")
  } else {
    temp_dir.path().join("binary")
  };

  // try with and without --node-modules-dir
  let compile_commands = &[
    "compile --output binary main.ts",
    "compile --node-modules-dir --output binary main.ts",
  ];

  for compile_command in compile_commands {
    let output = context.new_command().args(compile_command).run();
    output.assert_exit_code(0);
    output.skip_output_check();

    let output = context.new_command().name(&binary_path).run();
    output.assert_matches_text(
      r#"Node esm importing node cjs
===========================
{
  default: [Function (anonymous)],
  named: [Function (anonymous)],
  MyClass: [class MyClass]
}
{ default: [Function (anonymous)], named: [Function (anonymous)] }
[Module: null prototype] {
  MyClass: [class MyClass],
  __esModule: true,
  default: {
    default: [Function (anonymous)],
    named: [Function (anonymous)],
    MyClass: [class MyClass]
  },
  named: [Function (anonymous)]
}
[Module: null prototype] {
  __esModule: true,
  default: { default: [Function (anonymous)], named: [Function (anonymous)] },
  named: [Function (anonymous)]
}
===========================
static method
testing[WILDCARD]this
2
5
"#,
    );
  }

  // try with a package.json
  temp_dir.remove_dir_all("node_modules");
  temp_dir.write(
    "main.ts",
    concat!(
      "import { getValue, setValue } from '@denotest/esm-basic';\n",
      "setValue(2);\n",
      "console.log(getValue());",
    ),
  );
  temp_dir.write(
    "package.json",
    r#"{ "dependencies": { "@denotest/esm-basic": "1" } }"#,
  );

  let output = context
    .new_command()
    .args("compile --output binary main.ts")
    .run();
  output.assert_exit_code(0);
  output.skip_output_check();

  let output = context.new_command().name(binary_path).run();
  output.assert_matches_text("2\n");
}

#[test]
fn compile_npm_file_system() {
  run_npm_bin_compile_test(RunNpmBinCompileOptions {
    input_specifier: "compile/npm_fs/main.ts",
    compile_args: vec!["-A"],
    run_args: vec![],
    output_file: "compile/npm_fs/main.out",
    node_modules_dir: true,
    input_name: Some("binary"),
    expected_name: "binary",
    exit_code: 0,
  });
}

#[test]
fn compile_npm_bin_esm() {
  run_npm_bin_compile_test(RunNpmBinCompileOptions {
    input_specifier: "npm:@denotest/bin/cli-esm",
    compile_args: vec![],
    run_args: vec!["this", "is", "a", "test"],
    output_file: "npm/deno_run_esm.out",
    node_modules_dir: false,
    input_name: None,
    expected_name: "cli-esm",
    exit_code: 0,
  });
}

#[test]
fn compile_npm_bin_cjs() {
  run_npm_bin_compile_test(RunNpmBinCompileOptions {
    input_specifier: "npm:@denotest/bin/cli-cjs",
    compile_args: vec![],
    run_args: vec!["this", "is", "a", "test"],
    output_file: "npm/deno_run_cjs.out",
    node_modules_dir: false,
    input_name: None,
    expected_name: "cli-cjs",
    exit_code: 0,
  });
}

#[test]
fn compile_npm_cowsay_main() {
  run_npm_bin_compile_test(RunNpmBinCompileOptions {
    input_specifier: "npm:cowsay@1.5.0",
    compile_args: vec!["--allow-read"],
    run_args: vec!["Hello"],
    output_file: "npm/deno_run_cowsay.out",
    node_modules_dir: false,
    input_name: None,
    expected_name: "cowsay",
    exit_code: 0,
  });
}

#[test]
fn compile_npm_vfs_implicit_read_permissions() {
  run_npm_bin_compile_test(RunNpmBinCompileOptions {
    input_specifier: "compile/vfs_implicit_read_permission/main.ts",
    compile_args: vec![],
    run_args: vec![],
    output_file: "compile/vfs_implicit_read_permission/main.out",
    node_modules_dir: false,
    input_name: Some("binary"),
    expected_name: "binary",
    exit_code: 0,
  });
}

#[test]
fn compile_npm_no_permissions() {
  run_npm_bin_compile_test(RunNpmBinCompileOptions {
    input_specifier: "npm:cowsay@1.5.0",
    compile_args: vec![],
    run_args: vec!["Hello"],
    output_file: "npm/deno_run_cowsay_no_permissions.out",
    node_modules_dir: false,
    input_name: None,
    expected_name: "cowsay",
    exit_code: 1,
  });
}

#[test]
fn compile_npm_cowsay_explicit() {
  run_npm_bin_compile_test(RunNpmBinCompileOptions {
    input_specifier: "npm:cowsay@1.5.0/cowsay",
    compile_args: vec!["--allow-read"],
    run_args: vec!["Hello"],
    output_file: "npm/deno_run_cowsay.out",
    node_modules_dir: false,
    input_name: None,
    expected_name: "cowsay",
    exit_code: 0,
  });
}

#[test]
fn compile_npm_cowthink() {
  run_npm_bin_compile_test(RunNpmBinCompileOptions {
    input_specifier: "npm:cowsay@1.5.0/cowthink",
    compile_args: vec!["--allow-read"],
    run_args: vec!["Hello"],
    output_file: "npm/deno_run_cowthink.out",
    node_modules_dir: false,
    input_name: None,
    expected_name: "cowthink",
    exit_code: 0,
  });
}

struct RunNpmBinCompileOptions<'a> {
  input_specifier: &'a str,
  node_modules_dir: bool,
  output_file: &'a str,
  input_name: Option<&'a str>,
  expected_name: &'a str,
  run_args: Vec<&'a str>,
  compile_args: Vec<&'a str>,
  exit_code: i32,
}

fn run_npm_bin_compile_test(opts: RunNpmBinCompileOptions) {
  let context = TestContextBuilder::for_npm().use_temp_cwd().build();

  let temp_dir = context.temp_dir();
  let main_specifier = if opts.input_specifier.starts_with("npm:") {
    opts.input_specifier.to_string()
  } else {
    testdata_path().join(opts.input_specifier).to_string()
  };

  let mut args = vec!["compile".to_string()];

  args.extend(opts.compile_args.iter().map(|s| s.to_string()));

  if opts.node_modules_dir {
    args.push("--node-modules-dir".to_string());
  }

  if let Some(bin_name) = opts.input_name {
    args.push("--output".to_string());
    args.push(bin_name.to_string());
  }

  args.push(main_specifier);

  // compile
  let output = context.new_command().args_vec(args).run();
  output.assert_exit_code(0);
  output.skip_output_check();

  // delete the npm folder in the DENO_DIR to ensure it's not using it
  context.deno_dir().remove_dir_all("./npm");

  // run
  let binary_path = if cfg!(windows) {
    temp_dir.path().join(format!("{}.exe", opts.expected_name))
  } else {
    temp_dir.path().join(opts.expected_name)
  };
  let output = context
    .new_command()
    .name(binary_path)
    .args_vec(opts.run_args)
    .run();
  output.assert_matches_file(opts.output_file);
  output.assert_exit_code(opts.exit_code);
}

#[test]
fn compile_node_modules_symlink_outside() {
  let context = TestContextBuilder::for_npm()
    .use_copy_temp_dir("compile/node_modules_symlink_outside")
    .cwd("compile/node_modules_symlink_outside")
    .build();

  let temp_dir = context.temp_dir();
  let project_dir = temp_dir
    .path()
    .join("compile")
    .join("node_modules_symlink_outside");
  temp_dir.create_dir_all(project_dir.join("node_modules"));
  temp_dir.create_dir_all(project_dir.join("some_folder"));
  temp_dir.write(project_dir.join("test.txt"), "5");

  // create a symlink in the node_modules directory that points to a folder in the cwd
  temp_dir.symlink_dir(
    project_dir.join("some_folder"),
    project_dir.join("node_modules").join("some_folder"),
  );
  // compile folder
  let output = context
    .new_command()
    .args("compile --allow-read --node-modules-dir --output bin main.ts")
    .run();
  output.assert_exit_code(0);
  output.assert_matches_file(
    "compile/node_modules_symlink_outside/main_compile_folder.out",
  );
  assert!(project_dir.join("node_modules/some_folder").exists());

  // Cleanup and remove the folder. The folder test is done separately from
  // the file symlink test because different systems would traverse
  // the directory items in different order.
  temp_dir.remove_dir_all(project_dir.join("node_modules/some_folder"));

  // create a symlink in the node_modules directory that points to a file in the cwd
  temp_dir.symlink_file(
    project_dir.join("test.txt"),
    project_dir.join("node_modules").join("test.txt"),
  );
  assert!(project_dir.join("node_modules/test.txt").exists());

  // compile
  let output = context
    .new_command()
    .args("compile --allow-read --node-modules-dir --output bin main.ts")
    .run();
  output.assert_exit_code(0);
  output.assert_matches_file(
    "compile/node_modules_symlink_outside/main_compile_file.out",
  );

  // run
  let binary_path =
    project_dir.join(if cfg!(windows) { "bin.exe" } else { "bin" });
  let output = context.new_command().name(binary_path).run();
  output.assert_matches_file("compile/node_modules_symlink_outside/main.out");
}
