// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use deno_core::unsync::spawn;
use tokio::time::sleep;
use tokio::time::Duration;

/// Starts a task that will check for the existence of the
/// provided process id. Once that process no longer exists
/// it will terminate the current process.
pub fn start(parent_process_id: u32) {
  spawn(async move {
    loop {
      sleep(Duration::from_secs(30)).await;

      if !is_process_active(parent_process_id) {
        std::process::exit(1);
      }
    }
  });
}

#[cfg(unix)]
fn is_process_active(process_id: u32) -> bool {
  // TODO(bartlomieju):
  #[allow(clippy::undocumented_unsafe_blocks)]
  unsafe {
    // signal of 0 checks for the existence of the process id
    libc::kill(process_id as i32, 0) == 0
  }
}

#[cfg(windows)]
fn is_process_active(process_id: u32) -> bool {
  use winapi::shared::minwindef::DWORD;
  use winapi::shared::minwindef::FALSE;
  use winapi::shared::ntdef::NULL;
  use winapi::shared::winerror::WAIT_TIMEOUT;
  use winapi::um::handleapi::CloseHandle;
  use winapi::um::processthreadsapi::OpenProcess;
  use winapi::um::synchapi::WaitForSingleObject;
  use winapi::um::winnt::SYNCHRONIZE;

  // SAFETY: winapi calls
  unsafe {
    let process = OpenProcess(SYNCHRONIZE, FALSE, process_id as DWORD);
    let result = if process == NULL { false } else { WaitForSingleObject(process, 0) == WAIT_TIMEOUT };
    CloseHandle(process);
    result
  }
}


