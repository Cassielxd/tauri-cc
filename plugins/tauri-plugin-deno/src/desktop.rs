use std::{collections::HashMap, sync::Arc};

use deno_lib::deno_ipc::{messages::IpcMessage, IpcReceiver, IpcSender};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::RwLock;

use crate::{models::*, DenoExt};

pub fn init<R: Runtime>(app: &AppHandle<R>, main_module: String) -> crate::Result<DenoManager<R>> {
  let deno_manager = DenoManager::new(app.clone(), main_module);
  let _ = deno_manager.initialize();
  Ok(deno_manager)
}
///deno 插件管理器
/// workers_table deno 进程的map
/// main_module deno 主进程的模块
#[derive(Clone)]
pub struct DenoManager<R: Runtime> {
  pub handler: AppHandle<R>,
  pub main_module: String,
  pub deno_sender: IpcSender,
  pub deno_receiver: IpcReceiver,
  pub workers_table: Arc<RwLock<HashMap<String, WorkerManager>>>,
}
impl<R: Runtime> DenoManager<R> {
  pub fn new(handler: AppHandle<R>, main_module: String) -> Self {
    let (deno_sender, deno_receiver) = async_channel::unbounded::<IpcMessage>();

    Self {
      handler,
      main_module,
      deno_sender,
      deno_receiver,
      workers_table: Arc::new(RwLock::new(HashMap::new())),
    }
  }
  ///初始化插件并启动 deno 进程
  pub fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
    let handle_ref = self.handler.clone();
    let reff = self.workers_table.clone();
    let main_module_ref = self.main_module.clone();
    let deno_sender_ref = self.deno_sender.clone();
    //初始化主deno线程
    tokio::task::spawn(async move {
      let key = "main".to_string();
      reff.write().await.insert(key.clone(), WorkerManager::new(key, main_module_ref, deno_sender_ref));
      run(handle_ref).await;
    });
    Ok(())
  }
}

/// deno 插件运行主函数
/// 通信实现
/// 1.接收webview发来的消息，通过webview id找到对应的worker，然后通知worker
/// 2.接收deno发来的消息，通过id找到对应的worker，然后通知worker
async fn run<R: Runtime>(handle_ref: tauri::AppHandle<R>) {
  let ipc_recever = handle_ref.receiver();
  let workers_table_ref = handle_ref.workers_table();
  loop {
    match ipc_recever.recv().await.unwrap() {
      IpcMessage::SentToWindow(msg) => {
        let window = handle_ref.get_webview_window(&msg.id);
        match window {
          Some(window) => {
            let _ = window.emit(&msg.event, msg.content);
          }
          None => {
            let _ = handle_ref.emit(&msg.event, msg.content);
          }
        }
      }
      IpcMessage::SentToDeno(msg) => {
        let events_manager_map = workers_table_ref.read().await;
        match events_manager_map.get(&msg.id) {
          Some(worker_manager) => {
            //通知指定的worker
            worker_manager.events_manager.send(msg.event.clone(), msg.content).await.unwrap();
          }
          None => {
            //通知所有的worker
            for (_key, worker_manager) in events_manager_map.iter() {
              worker_manager.events_manager.send(msg.event.clone(), msg.content.clone()).await.unwrap();
            }
          }
        }
      }
    }
  }
}
