use deno_core::{JsBuffer, op2, OpState, ResourceId};
use deno_core::error::{AnyError, custom_error};
use deno_core::serde_json::Value;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use crate::in_memory_event_channel::{InMemoryBroadcastChannel, InMemoryBroadcastChannelResource};

#[derive(Deserialize, Serialize)]
pub struct EmitPayload {
    pub event: String,
    pub data: Option<Value>,
}

#[op2]
#[serde]
fn emit(state: &mut OpState, #[string] id: String, #[serde] emit: EmitPayload) -> Result<(), AnyError> {
    let app_handle = state.borrow_mut::<AppHandle>();
    let window = app_handle.get_window(&*id);
    match window {
        Some(win) => {
            let _ = win.emit(&emit.event, emit.data);
            Ok(())
        }
        None => Err(custom_error("Tauri", "Window not found"))
    }
}

#[op2(fast)]
pub fn emit_all(state: &mut OpState,
                #[smi] rid: ResourceId,
                #[string] name: String,
                #[buffer] buf: JsBuffer, ) -> Result<(), AnyError> {
    let app_handle = state.borrow::<AppHandle>();

    let data = serde_json::from_slice::<Value>(&buf.clone().to_vec()).unwrap();
    {
        let bc = app_handle.state::<InMemoryBroadcastChannel>().clone();
        let resource = state.resource_table.get::<InMemoryBroadcastChannelResource>(rid)?;
        let _ = bc.send(&resource, name.clone(), buf.to_vec());
    }
    let _ = app_handle.emit_all(&name, &data);
    Ok(())
}