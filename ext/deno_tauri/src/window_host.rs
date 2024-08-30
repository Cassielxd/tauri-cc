use std::collections::HashMap;
use deno_core::error::{custom_error, AnyError, type_error};
use deno_core::{DetachedBuffer, op2, OpState, RcRef, ResourceId};

use tauri::{AppHandle, Manager, Window, WindowUrl};


pub struct TauriWindow {
    win: Window,
}

pub type WindowsTable = HashMap<String, TauriWindow>;

deno_core::extension!(
  deno_tauri_window_host,
  ops = [create_window,close_window],
  state = |state| {
    state.put::<WindowsTable>(WindowsTable::default());
  },
);


#[op2]
#[serde]
fn create_window(state: &mut OpState, #[string] id: String, #[serde] url: WindowUrl) -> Result<(), AnyError> {
    let app_handle = state.borrow_mut::<AppHandle>();
    //如果已经存在则不创建
    if let Some(win) = app_handle.get_window(&*id.clone()) {
        let windows = state.borrow_mut::<WindowsTable>();
        if !windows.contains_key(&id) {
            windows.insert(id, TauriWindow { win });
        }
    } else {
        match tauri::WindowBuilder::new(app_handle, id.clone(), url).build() {
            Ok(win) => {
                state.borrow_mut::<WindowsTable>().insert(id, TauriWindow { win });
            }
            Err(err) => {
                println!("window creation failed: {}", err);
            }
        }
    }
    Ok(())
}

#[op2]
#[serde]
fn close_window(state: &mut OpState, #[string] id: String) -> Result<(), AnyError> {
    if let Some(window) = state.borrow_mut::<WindowsTable>().remove(&id) {
        let _ = window.win.close();
    } else {
        println!("window not found: {}", id);
    }
    Ok(())
}



