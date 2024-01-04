// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use deno_core::Snapshot;
use log::debug;

#[cfg(not(feature = "__runtime_js_sources"))]
static CLI_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/CLI_SNAPSHOT.bin"));

pub fn deno_isolate_init() -> Option<Snapshot> {
  debug!("Deno isolate init with snapshots.");
  #[cfg(not(feature = "__runtime_js_sources"))]
  {
    Some(Snapshot::Static(CLI_SNAPSHOT))
  }
  #[cfg(feature = "__runtime_js_sources")]
  {
    None
  }
}
