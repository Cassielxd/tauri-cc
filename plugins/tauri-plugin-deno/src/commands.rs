

use tauri::{ipc::Channel, Manager, Resource, ResourceId, Runtime};

use deno_pro_lib::deno_ipcs::events_manager::EventsManager;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::DenoExt;

struct DenoResource{
    pub events_manager: EventsManager,
    pub on_event: Channel<serde_json::Value>,
}
impl DenoResource {
    async fn listen_on(&self,name: String,listener_id: Uuid){
        let (listener, mut receiver) = mpsc::channel(1);
        self.events_manager.listen_on(name.clone(), listener_id,listener).await;
        let on_event_ref: Channel<serde_json::Value> = self.on_event.clone();
        tokio::task::spawn(async move {
            loop {
                let value = receiver.recv().await;
                let _ = on_event_ref.send(value.unwrap());
            }
        });
        
     }
     async fn send_message(&self,event:String,message:serde_json::Value){

        let _ = self.events_manager.send(event, message).await;
     }
     async fn unlisten_from(&self,name: String,listener_id: Uuid){
        self.events_manager.unlisten_from(name.clone(), listener_id).await;
     }
     
}
impl Resource for  DenoResource{
   
    fn name(&self) -> std::borrow::Cow<'_, str> {
        std::any::type_name::<Self>().into()
      }
    
    fn close(self: std::sync::Arc<Self>) {
       
    }

}

#[tauri::command]
pub async fn send_to_deno<R: Runtime>(window: tauri::Window<R>,name:String,  rid: ResourceId, content: serde_json::Value) {
   let channel = window.resources_table().get::<DenoResource>(rid);
   match channel {
    Ok(channel) => {
        channel.send_message(name, content).await;
    },
    Err(_) => {},
   }
}


#[tauri::command]
pub  fn create_deno_channel<R: Runtime>(window: tauri::Window<R>,key:String,on_event: Channel<serde_json::Value>)->ResourceId {
    let w_ref: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, crate::WorkerManager>>> =window.deno().workers_table.clone();
    let workers_table =w_ref.try_read().unwrap();
   if let Some(worker_manager) = workers_table.get(&key){
   let deno_channel = DenoResource{ events_manager:worker_manager.events_manager.clone(), on_event};
    return window.resources_table().add(deno_channel);
   }
    0
}
#[tauri::command]
pub async fn listen_on<R: Runtime>(window: tauri::Window<R>,rid: ResourceId, name: String)->Uuid{
    let channel = window.resources_table().get::<DenoResource>(rid).unwrap();
    let  listener_id= Uuid::new_v4();
        channel.listen_on(name,listener_id).await;
        return  listener_id;
}

#[tauri::command]
pub async fn unlisten_from<R: Runtime>(window: tauri::Window<R>,rid: ResourceId,name: String,listenerid:Uuid) {
    let deno_channel =window.resources_table().get::<DenoResource>(rid);
    match deno_channel {
    Ok(channel) => {
        channel.unlisten_from(name,listenerid).await;
    },
    Err(_) => {},
    }
}


#[tauri::command]
pub async fn close_deno_channel<R: Runtime>(window: tauri::Window<R>,rid: ResourceId) {
    let _ =window.resources_table().close(rid);
}