use std::cell::RefCell;
use std::rc::Rc;
use deno_core::{JsBuffer, op2, OpState, ResourceId};
use deno_core::error::AnyError;
use tauri::{AppHandle, Manager};
use crate::in_memory_event_channel::{InMemoryBroadcastChannel, InMemoryBroadcastChannelResource};

pub mod window;
pub mod emit;
pub mod event;
pub mod in_memory_event_channel;


pub type Message = (String, Vec<u8>);



deno_core::extension!(deno_tauri_broadcast_channel,
  deps = [ deno_webidl, deno_web ],
  ops = [
    op_tauri_broadcast_subscribe,
    op_tauri_broadcast_unsubscribe,
    op_tauri_broadcast_send,
    op_tauri_broadcast_recv,
  ],
  esm = [ dir "src","019_tauri_channel.js" ],
  options = {
    app_handle: AppHandle,
  },
  state = |state, options| {
    state.put(options.app_handle);
  },
);

#[op2(fast)]
#[smi]
pub fn op_tauri_broadcast_subscribe(
    state: &mut OpState,
) -> Result<ResourceId, AnyError>
{
    let app_handle = state.borrow::<AppHandle>().clone();
    let bc = app_handle.state::<InMemoryBroadcastChannel>().clone();
    let resource = bc.subscribe()?;
    Ok(state.resource_table.add(resource))
}

#[op2(async)]
#[serde]
pub async fn op_tauri_broadcast_recv(
    state: Rc<RefCell<OpState>>,
    #[smi] rid: ResourceId,
) -> Result<Option<Message>, AnyError>
{
    let resource = state.borrow().resource_table.get::<InMemoryBroadcastChannelResource>(rid)?;
    let app_handle = state.borrow().borrow::<AppHandle>().clone();
    let bc = app_handle.state::<InMemoryBroadcastChannel>().clone();
    bc.recv(&resource).await
}


#[op2(async)]
pub async fn op_tauri_broadcast_send(
    state: Rc<RefCell<OpState>>,
    #[smi] rid: ResourceId,
    #[string] name: String,
    #[buffer] buf: JsBuffer,
) -> Result<(), AnyError>
{
    //let data = serde_json::from_slice::<serde_json::Value>(&buf.clone().to_vec()).unwrap();
    let resource = state.borrow().resource_table.get::<InMemoryBroadcastChannelResource>(rid)?;
    let app_handle = state.borrow().borrow::<AppHandle>().clone();
    let bc = app_handle.state::<InMemoryBroadcastChannel>().clone();
    bc.send(&resource, name, buf.to_vec())
}

#[op2(fast)]
pub fn op_tauri_broadcast_unsubscribe(
    state: &mut OpState,
    #[smi] rid: ResourceId,
) -> Result<(), AnyError>
{
    let resource = state.resource_table.get::<InMemoryBroadcastChannelResource>(rid)?;
    let app_handle = state.borrow::<AppHandle>();
    let bc = app_handle.state::<InMemoryBroadcastChannel>().clone();
    bc.unsubscribe(&resource)
}