use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tauri::{ipc::Channel, Emitter, Manager, Resource, ResourceId, Runtime};

use deno_pro_lib::deno_ipcs::{
  events_manager::EventsManager,
  messages::{IpcMessage, SentToDenoMessage},
};
use tokio::{
  select,
  sync::{
    mpsc::{channel, Sender},
    Mutex,
  },
};
use uuid::Uuid;

use crate::DenoExt;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ChannelMessage {
  pub event: String, //对应的事件
  pub content: serde_json::Value,
}
type ResouceMap = Arc<Mutex<HashMap<String, Sender<bool>>>>;
//DenoResource 通信默认实现
struct DenoResource<R: Runtime> {
  pub webview: tauri::Webview<R>,
  pub events_manager: EventsManager,
  pub on_event: Channel<ChannelMessage>,
  pub resouce_map: ResouceMap,
}
impl<R: Runtime> DenoResource<R> {
  //事件监听
  async fn listen_on(&self, name: String) {
    let mut map = self.resouce_map.lock().await;
    if map.contains_key(&name) {
      return;
    }
    let name_ref = name.clone();
    let (listener, mut receiver) = channel(1);
    let (resource_sender, mut resource_receiver) = channel::<bool>(1);
    let events_manager_ref = self.events_manager.clone();
    let on_event_ref: Channel<ChannelMessage> = self.on_event.clone();
    tokio::task::spawn(async move {
      let listener_id = Uuid::new_v4();
      events_manager_ref.listen_on(name.clone(), listener_id, listener).await;
      loop {
        select! {
            value = receiver.recv() => {
             
                let result = on_event_ref.send(ChannelMessage{event:name.clone(),content:value.unwrap()});
                
                if let Err(e) = result{
                  println!("send_message_to_deno_error:{:?}",e);
                    events_manager_ref.unlisten_from(name.clone(), listener_id).await;
                    break;
                }
            },
            _ = resource_receiver.recv() => {
                events_manager_ref.unlisten_from(name.clone(), listener_id).await;
                break;
            }
        }
      }
    });
    map.insert(name_ref, resource_sender);
  }
  //发送消息
  async fn send_message(&self, event: String, message: serde_json::Value) {
    let _ = self.events_manager.send(event, message).await;
  }
  //取消监听
  async fn unlisten_from(&self, name: String) {
    let resounce = self.resouce_map.lock().await.remove(&name);
    if let Some(r) = resounce {
      let _ = r.send(true).await;
    }
  }
}
impl<R:Runtime> Resource for DenoResource<R> {
  fn name(&self) -> std::borrow::Cow<'_, str> {
    std::any::type_name::<Self>().into()
  }

  fn close(self: std::sync::Arc<Self>) {}
}
/// 向所有deno 发送消息
#[tauri::command]
pub async fn send_to_all_deno<R: Runtime>(window: tauri::Webview<R>, name: String, content: serde_json::Value) {
  let w_ref = window.sender();
  let _ = w_ref.send(IpcMessage::SentToDeno(SentToDenoMessage { id: "".to_string(), event: name, content })).await;
}

// Deno命令 向指定的deno 发送消息
#[tauri::command]
pub async fn send_to_deno<R: Runtime>(window: tauri::Webview<R>, name: String, rid: ResourceId, content: serde_json::Value) {
  let channel = window.resources_table().get::<DenoResource<R>>(rid);
  match channel {
    Ok(channel) => {
      channel.send_message(name, content).await;
    }
    Err(_) => {}
  }
}
#[tauri::command]
pub fn check_deno_channel<R: Runtime>(window: tauri::Webview<R>, key: String) -> bool {
  let w_ref: std::sync::Arc<tokio::sync::RwLock<HashMap<String, crate::WorkerManager>>> = window.workers_table();
  let workers_table: tokio::sync::RwLockReadGuard<'_, HashMap<String, crate::WorkerManager>> = w_ref.try_read().unwrap();
  match workers_table.get(&key) {
    Some(_) => true,
    None => false,
  }
}
// 于指定的deno 创建通道
#[tauri::command]
pub fn create_deno_channel<R: Runtime>(webview: tauri::Webview<R>, key: String, on_event: Channel<ChannelMessage>) -> ResourceId {
  let w_ref = webview.workers_table();
  let workers_table: tokio::sync::RwLockReadGuard<'_, HashMap<String, crate::WorkerManager>> = w_ref.try_read().unwrap();
  if let Some(worker_manager) = workers_table.get(&key) {
    let deno_channel = DenoResource {
      webview:webview.clone(),
      events_manager: worker_manager.events_manager.clone(),
      on_event,
      resouce_map: Arc::new(Mutex::new(HashMap::new())) ,
    };
    return webview.resources_table().add(deno_channel);
  }
  0
}
// 监听事件
#[tauri::command]
pub async fn listen_on<R: Runtime>(window: tauri::Webview<R>, rid: ResourceId, name: String) {

  let channel = window.resources_table().get::<DenoResource<R>>(rid).unwrap();
  channel.listen_on(name.clone()).await;
}
// 取消监听
#[tauri::command]
pub async fn unlisten_from<R: Runtime>(window: tauri::Webview<R>, rid: ResourceId, name: String) {
  let deno_channel = window.resources_table().get::<DenoResource<R>>(rid);
  match deno_channel {
    Ok(channel) => {
      channel.unlisten_from(name).await;
    }
    Err(_) => {}
  }
}
// 关闭通道
#[tauri::command]
pub async fn close_deno_channel<R: Runtime>(window: tauri::Webview<R>, rid: ResourceId) {
  let deno_channel = window.resources_table().take::<DenoResource<R>>(rid);
  match deno_channel {
    Ok(c) => {
      tokio::task::spawn(async move {
        let map = c.resouce_map.lock().await;
        for (_, v) in map.iter() {
          let _ = v.send(true).await;
        }
      });
    }
    Err(_) => {}
  }
}
