// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

pub mod args;
pub mod auth_tokens;
pub mod cache;
pub mod cdp;
pub mod deno_std;
pub mod emit;
pub mod errors;
pub mod factory;
pub mod file_fetcher;
pub mod graph_util;
pub mod http_util;
pub mod js;
pub mod lsp;
pub mod module_loader;
pub mod napi;
pub mod node;
pub mod npm;
pub mod ops;
pub mod resolver;
pub mod standalone;
pub mod tools;
pub mod tsc;
pub mod util;
pub mod version;
pub mod worker;

use crate::args::Flags;
use crate::util::display;
pub use deno_config;
pub use deno_runtime;
use deno_runtime::colors;
use factory::CliFactory;

// NOTE(bartlomieju): keep IDs in sync with `runtime/90_deno_ns.js`.
pub(crate) static UNSTABLE_GRANULAR_FLAGS: &[(
  // flag name
  &str,
  // help text
  &str,
  // id to enable it in runtime/99_main.js
  i32,
)] = &[
  (deno_runtime::deno_broadcast_channel::UNSTABLE_FEATURE_NAME, "Enable unstable `BroadcastChannel` API", 1),
  (deno_runtime::deno_ffi::UNSTABLE_FEATURE_NAME, "Enable unstable FFI APIs", 2),
  (deno_runtime::deno_fs::UNSTABLE_FEATURE_NAME, "Enable unstable file system APIs", 3),
  (deno_runtime::deno_kv::UNSTABLE_FEATURE_NAME, "Enable unstable Key-Value store APIs", 4),
  (deno_runtime::deno_net::UNSTABLE_FEATURE_NAME, "Enable unstable net APIs", 5),
  (deno_runtime::ops::http::UNSTABLE_FEATURE_NAME, "Enable unstable HTTP APIs", 6),
  (deno_runtime::ops::worker_host::UNSTABLE_FEATURE_NAME, "Enable unstable Web Worker APIs", 7),
  (deno_runtime::deno_cron::UNSTABLE_FEATURE_NAME, "Enable unstable Deno.cron API", 8),
];

pub(crate) fn unstable_exit_cb(_feature: &str, api_name: &str) {
  // TODO(bartlomieju): change to "The `--unstable-{feature}` flag must be provided.".
  eprintln!("Unstable API '{api_name}'. The --unstable flag must be provided.");
  std::process::exit(70);
}
