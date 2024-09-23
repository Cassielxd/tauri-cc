

#[cfg(desktop)]
mod desktop;
use desktop::DenoManager;
use tauri::{
    plugin::{Builder, TauriPlugin},  Manager, Runtime
};

use deno_pro_lib::deno_ipcs::{events_manager::EventsManager, IpcSender,IpcReceiver};
use state::Container;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
pub type Result<T> = std::result::Result<T, Error>;

pub use models::*;


mod commands;
mod error;
mod models;

pub use error::Error;



pub type WorkersTable =Mutex<HashMap<String, WorkerManager>>;

pub type ManagerMap= Arc<Mutex<HashMap<String,EventsManager>>>;

pub trait DenoExt<R: Runtime> {
    fn deno(&self) -> &DenoManager<R>;
    fn sender(&self) -> IpcSender;
    fn receiver(&self) -> IpcReceiver;
    fn workers_table(&self) -> Arc<RwLock<HashMap<String, WorkerManager>>>;
  }

  impl<R: Runtime, T: Manager<R>> crate::DenoExt<R> for T {
    fn deno(&self) -> &DenoManager<R> {
      self.state::<DenoManager<R>>().inner()
    }
    fn sender(&self) -> IpcSender{
      self.state::<DenoManager<R>>().inner().deno_sender.clone()
    }
    fn receiver(&self) -> IpcReceiver{
      self.state::<DenoManager<R>>().inner().deno_receiver.clone()
    }
    fn workers_table(&self) -> Arc<RwLock<HashMap<String, WorkerManager>>>{
      self.state::<DenoManager<R>>().inner().workers_table.clone()
    }
  }



  
/// Initializes the plugin.
pub fn init<R: Runtime>( main_module: String,) -> TauriPlugin<R> {
    Builder::new("deno")
      .invoke_handler(tauri::generate_handler![commands::check_deno_channel,commands::send_to_all_deno,commands::send_to_deno,commands::create_deno_channel,commands::unlisten_from,commands::listen_on,commands::close_deno_channel])
      .setup(|app, _api: tauri::plugin::PluginApi<R, ()>| {
        let app_ref =app.clone();
        #[cfg(desktop)]
        let deno =desktop::init(&app_ref,main_module)?;
        app.manage(deno);
        Ok(())
      })
      .build()
  }
  