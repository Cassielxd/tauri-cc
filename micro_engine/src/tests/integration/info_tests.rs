// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use test_util as util;
use util::env_vars_for_npm_tests;
use util::TestContextBuilder;

#[test]
fn info_with_compiled_source() {
  let context = TestContextBuilder::new().use_http_server().build();
  let module_path = "http://127.0.0.1:4545/run/048_media_types_jsx.ts";

  let output = context
    .new_command()
    .current_dir(util::testdata_path())
    .args_vec(["cache", module_path])
    .run();
  output.assert_exit_code(0);
  output.skip_output_check();

  let output = context
    .new_command()
    .current_dir(util::testdata_path())
    .args_vec(["info", module_path])
    .split_output()
    .run();

  // check the output of the test.ts program.
  assert!(output.stdout().trim().contains("emit: "));
  assert_eq!(output.stderr(), "");
}

itest!(multiple_imports {
  args: "info http://127.0.0.1:4545/run/019_media_types.ts",
  output: "info/multiple_imports.out",
  http_server: true,
});

itest!(info_ts_error {
  args: "info info/031_info_ts_error.ts",
  output: "info/031_info_ts_error.out",
});

itest!(info_flag {
  args: "info",
  output: "info/041_info_flag.out",
});

itest!(info_flag_location {
  args: "info --location https://deno.land",
  output: "info/041_info_flag_location.out",
});

itest!(info_json {
  args: "info --json --unstable",
  output: "info/info_json.out",
});

itest!(info_json_location {
  args: "info --json --unstable --location https://deno.land",
  output: "info/info_json_location.out",
});

itest!(info_flag_script_jsx {
  args: "info http://127.0.0.1:4545/run/048_media_types_jsx.ts",
  output: "info/049_info_flag_script_jsx.out",
  http_server: true,
});

itest!(json_file {
  args: "info --quiet --json --unstable info/json_output/main.ts",
  output: "info/json_output/main.out",
  exit_code: 0,
});

itest!(import_map_info {
  args:
    "info --quiet --import-map=import_maps/import_map.json import_maps/test.ts",
  output: "info/065_import_map_info.out",
});

itest!(info_json_deps_order {
  args: "info --unstable --json info/076_info_json_deps_order.ts",
  output: "info/076_info_json_deps_order.out",
});

itest!(info_missing_module {
  args: "info info/error_009_missing_js_module.js",
  output: "info/info_missing_module.out",
});

itest!(info_lock {
  args: "info main.ts",
  http_server: true,
  cwd: Some("lockfile/basic"),
  exit_code: 10,
  output: "lockfile/basic/fail.out",
});

itest!(info_no_lock {
  args: "info --no-lock main.ts",
  http_server: true,
  cwd: Some("lockfile/basic"),
  output: "lockfile/basic/info.nolock.out",
});

itest!(info_recursive_modules {
  args: "info --quiet info/info_recursive_imports_test.ts",
  output: "info/info_recursive_imports_test.out",
  exit_code: 0,
});

itest!(info_type_import {
  args: "info info/info_type_import.ts",
  output: "info/info_type_import.out",
});

itest!(_054_info_local_imports {
  args: "info --quiet run/005_more_imports.ts",
  output: "info/054_info_local_imports.out",
  exit_code: 0,
});

// Tests for AssertionError where "data" is unexpectedly null when
// a file contains only triple slash references (#11196)
itest!(data_null_error {
  args: "info info/data_null_error/mod.ts",
  output: "info/data_null_error/data_null_error.out",
});

itest!(types_header_direct {
  args: "info --reload run/type_directives_01.ts",
  output: "info/types_header.out",
  http_server: true,
});

itest!(with_config_override {
  args: "info info/with_config/test.ts --config info/with_config/deno-override.json --import-map info/with_config/import_map.json",
  output: "info/with_config/with_config.out",
});

itest!(package_json_basic {
  args: "info --quiet main.ts",
  output: "package_json/basic/main.info.out",
  envs: env_vars_for_npm_tests(),
  http_server: true,
  cwd: Some("package_json/basic"),
  copy_temp_dir: Some("package_json/basic"),
  exit_code: 0,
});

itest!(info_import_map {
  args: "info preact/debug",
  output: "info/with_import_map/with_import_map.out",
  cwd: Some("info/with_import_map"),
  exit_code: 0,
});
