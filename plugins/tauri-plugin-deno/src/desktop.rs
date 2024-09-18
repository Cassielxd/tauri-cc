use std::{collections::HashMap, sync::Arc};

use deno_pro_lib::deno_ipcs::{ messages::IpcMessage, IpcReceiver, IpcSender};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::RwLock;

use crate::{models::*, DenoExt};

pub  fn  init<R: Runtime>(
  app: &AppHandle<R>,
  main_module: String,
) -> crate::Result<DenoManager<R>> {
  let deno_manager = DenoManager::new(app.clone(),main_module);
     let _ = deno_manager.initialize();
  Ok(deno_manager)
}

#[derive(Clone)]
pub struct DenoManager<R: Runtime> {
  pub handler:AppHandle<R>,
  pub main_module: String,
  pub deno_sender: IpcSender,
  pub deno_receiver: IpcReceiver,
  pub workers_table:Arc<RwLock<HashMap<String, WorkerManager>>>
}
impl <R: Runtime> DenoManager<R>  {
  pub fn new( handler:AppHandle<R>,main_module: String) -> Self {
  let (deno_sender,deno_receiver) =async_channel::unbounded::<IpcMessage>();

    Self {
      handler,
      main_module,
      deno_sender,
      deno_receiver,
      workers_table: Arc::new(RwLock::new(HashMap::new())),
    }
  }
  pub  fn initialize(&self) ->Result<(), Box<dyn std::error::Error>> {
    let handle_ref= self.handler.clone();
    let reff = self.workers_table.clone();
    let main_module_ref=self.main_module.clone();
    let deno_sender_ref= self.deno_sender.clone();
    //初始化主deno线程
    tokio::task::spawn(   async move{
      reff.write().await.insert("main".to_string(), WorkerManager::new(main_module_ref,deno_sender_ref));
      run(handle_ref).await;
    });
    Ok(())
  }
}


async fn run<R: Runtime>(handle_ref: tauri::AppHandle<R>) {
  let ipc_recever: async_channel::Receiver<IpcMessage> =handle_ref.state::<IpcReceiver>().inner().clone();
  let workers_table_ref = handle_ref.deno().workers_table.clone();
  loop {
      match ipc_recever.recv().await.unwrap() {
          IpcMessage::SentToWindow(msg) => {
              
              let window = handle_ref.get_webview_window(&msg.id);
              match window {
                  Some(window) => {
                      let _ = window.emit(&msg.event, msg.content);
                  },
                  None => {
                      let _ = handle_ref.emit(&msg.event, msg.content);
                  },
              }
          },
          IpcMessage::SentToDeno(key,name, content) => {        
              let events_manager_map = workers_table_ref.read().await;
              match events_manager_map.get(&key) {
               Some(worker_manager) =>{
                   //通知指定的worker
                   worker_manager.events_manager
                   .send(name, content.clone())
                   .await
                   .unwrap();
               },
               None => {
                   //通知所有的worker
                   for (_key,worker_manager) in  events_manager_map.iter() {
                    worker_manager
                       .events_manager.send(name.clone(), content.clone())
                       .await
                       .unwrap();
                   }
               },
               }
          },
      }
  }
}
