use deno_core::{op2, OpState};
use deno_core::error::AnyError;
use tauri::{AppHandle, Manager};


#[op2]
#[serde]
fn listen_global(state: &mut OpState, #[string] event: String) -> Result<(), AnyError> {
    let app_handle = state.borrow_mut::<AppHandle>();

    app_handle.listen_global(event, |e| {});
    Ok(())
}