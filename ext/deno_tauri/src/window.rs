use deno_core::{op2, OpState};
use deno_core::error::{AnyError, custom_error};
use tauri::{AppHandle, Manager, WindowUrl};

#[op2]
#[serde]
fn create_window(state: &mut OpState, #[string] id: String, #[serde] url: WindowUrl) -> Result<(), AnyError> {
    let app_handle = state.borrow_mut::<AppHandle>();
    let _ = tauri::WindowBuilder::new(
        app_handle,
        id, /* the unique window label */
        url,
    ).build();
    Ok(())
}

#[op2]
#[serde]
fn close_window(state: &mut OpState, #[string] id: String) -> Result<(), AnyError> {
    let app_handle = state.borrow_mut::<AppHandle>();
    match app_handle.get_window(&*id) {
        Some(win) => {
            let _ = win.close();
            Ok(())
        }
        None => Err(custom_error("Tauri", "Window not found"))
    }
}