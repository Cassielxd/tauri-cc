
use serde_json::json;
use std::net::SocketAddr;
use std::str::FromStr;
use tauri::{
    plugin::{Builder, Plugin, Result as PluginResult, TauriPlugin}, window, AppHandle, Invoke, Manager, PageLoadPayload, Runtime, Window
};



use deno_pro_lib::{args::flags_from_vec, deno_ipcs::events_manager};
use deno_pro_lib::deno_ipcs::{deno_ipcs,IpcReceiver,IpcSender,messages::IpcMessage,events_manager::EventsManager};
use deno_pro_lib::deno_runtime::deno_core::v8;
use deno_pro_lib::deno_runtime::deno_permissions::PermissionsContainer;
use deno_pro_lib::deno_runtime::tokio_util::create_and_run_current_thread;
use deno_pro_lib::deno_runtime::WorkerExecutionMode;
use deno_pro_lib::factory::CliFactory;
use deno_pro_lib::tools::run::maybe_npm_install;
use futures:: task::AtomicWaker;
use serde::{ser, Deserialize};
use serde::Serialize;
use state::Container;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;
use tokio::{select, sync::{mpsc, Mutex}};

pub static APPLICATION_CONTEXT: Container![Send + Sync] = <Container![Send + Sync]>::new();
type WorkersTable =Mutex<HashMap<String, WorkersTableManager>>;
#[derive(Clone)]
pub struct WorkersTableManager {
    pub main_worker_thread: Arc<Mutex<Vec<MainWorkerThread>>>,
    pub main_nodule: String,
    pub worker_count: usize,
}

impl WorkersTableManager {
    pub async fn restart(self,deno_sender: IpcSender,events_manager: EventsManager,) {
        let mut stable = self.main_worker_thread.lock().await;
        *stable = {
            let mut arr = Vec::new();
            for i in 0..self.worker_count {
                arr.push(MainWorkerThread::new(self.main_nodule.clone(), deno_sender.clone(), events_manager.clone(),i));
            }
            arr
        }
    }
    pub fn new(main_nodule: String,deno_sender: IpcSender,events_manager: EventsManager, worker_count: usize) -> Self {
        WorkersTableManager {
            main_worker_thread: {
                let mut arr = Vec::new();
                for i in 0..worker_count {
                    arr.push(MainWorkerThread::new(main_nodule.clone(), deno_sender.clone(), events_manager.clone(),i));
                }
                Arc::new(Mutex::new(arr))
            },
            main_nodule,
            worker_count,
        }
    }
}

#[derive(Clone)]
pub struct MainWorkerHandle {
    sender: async_channel::Sender<u8>,
    // 发送器
    termination_signal: Arc<AtomicBool>,
    // 终止信号，使用Arc来实现共享和同步访问
    has_terminated: Arc<AtomicBool>,
    // 是否已经终止，使用Arc来实现共享和同步访问
    terminate_waker: Arc<AtomicWaker>,
    // 终止唤醒器，使用Arc来实现共享和同步访问
    isolate_handle: v8::IsolateHandle, // JavaScript Isolate 的句柄
}

impl MainWorkerHandle {
    pub fn terminate(self) {
        use std::thread::sleep;
        use std::thread::spawn;
        use std::time::Duration;
        // 是否需要安排终止
        let schedule_termination = !self.termination_signal.swap(true, Ordering::SeqCst);
        // 如果需要终止且尚未终止
        if schedule_termination && !self.has_terminated.load(Ordering::SeqCst) {
            // 唤醒任务的事件循环，以便完成终止操作
            self.terminate_waker.wake();

            let has_terminated = self.has_terminated.clone();

            // 安排终止JavaScript Isolate的执行
            spawn(move || {
                // 等待2秒
                sleep(Duration::from_secs(2));

                // 工作隔离环境只能终止一次，因此需要在此处添加一个保护机制
                let already_terminated = has_terminated.swap(true, Ordering::SeqCst);

                if !already_terminated {
                    // 如果尚未终止，则停止JavaScript执行
                    self.isolate_handle.terminate_execution();
                }
            });
        }
    }
}
pub struct MainWorkerThread {
    worker_handle: MainWorkerHandle,
}

impl MainWorkerThread {
    fn new(main_path: String, deno_sender: IpcSender,events_manager: EventsManager,index: usize) -> MainWorkerThread {
        // 创建一个用于线程间通信的同步通道
        let (handle_sender, handle_receiver) = sync_channel::<MainWorkerHandle>(1);
        // 创建一个线程，并为其命名
        let build = thread::Builder::new().name(format!("js-engine-{}", index));
        // 隐藏的线程任务，用于执行JavaScript引擎的初始化和运行"resource/main.ts".into()
        let _ = build.spawn(|| {
            let args = vec!["".to_string().into(), "run".to_string().into(), "--unstable".to_string().into(), "--inspect".to_string().into(), main_path.into()];
            // 将args转换为flagset
            let flags = Arc::new(flags_from_vec(args).unwrap());
            let future = async {
                let factory = CliFactory::from_flags(flags);
                let cli_options = factory.cli_options().unwrap();
                // 解析主模块
                let main_module = cli_options.resolve_main_module().unwrap();
                // 运行npm install
                maybe_npm_install(&factory).await.unwrap();
                // 创建CLI主工作线程工厂实例
                let worker_factory = factory.create_cli_main_worker_factory().await.unwrap();
                
                // 创建自定义工作线程实例
                let mut main_worker = worker_factory
                    .create_custom_worker(WorkerExecutionMode::Run, main_module, PermissionsContainer::allow_all(), vec![deno_ipcs::init_ops_and_esm(deno_sender,events_manager)], Default::default())
                    .await
                    .unwrap();
                // 获取工作线程的JavaScript运行时线程安全句柄
                let handle = main_worker.worker.js_runtime.v8_isolate().thread_safe_handle();
                let (sender, receiver) = async_channel::bounded::<u8>(1);
                // 创建一个MainWorkerHandle实例
                let external_handle = MainWorkerHandle {
                    sender,
                    termination_signal: Arc::new(AtomicBool::new(false)),
                    has_terminated: Arc::new(AtomicBool::new(false)),
                    terminate_waker: Arc::new(AtomicWaker::new()),
                    isolate_handle: handle,
                };
                // 发送MainWorkerHandle实例到handle_sender通道
                handle_sender.send(external_handle).unwrap();
                drop(handle_sender);
                // 选择执行不同的分支 有一个返回线程结束
                select! {
          res = receiver.recv() => {
            println!("结束了{:?}",res);
          }
          code = main_worker.run() => {
            println!("run {:?}",code);
           }
          }
        };
            // 创建并运行当前线程
            create_and_run_current_thread(future);
        });
        // 获取handle_receiver通道接收到的值，即MainWorkerHandle实例
        let worker_handle = handle_receiver.recv().unwrap();
        // 创建MainWorkerThread实例
        MainWorkerThread { worker_handle: worker_handle.into() }
    }
}

impl Drop for MainWorkerThread {
    fn drop(&mut self) {
        self.worker_handle.clone().terminate();
        self.worker_handle.clone().sender.send_blocking(1).expect("error");
    }
}
#[derive(Serialize, Deserialize)]
pub struct CommandStatus {
    status: bool,
    message: Option<String>,
}
#[tauri::command]
async fn start_engine<R: Runtime>(app: tauri::AppHandle<R>, key: String, path: String, worker_count: Option<usize>) -> CommandStatus {
    let worker_table = APPLICATION_CONTEXT.get::<WorkersTable>();
    let mut stable = worker_table.lock().await;
    if stable.contains_key(&key) {
        return CommandStatus {
            status: false,
            message: Some(format!("worker {} Already exist", key)),
        };
    }
    let ipc_sender =app.state::<IpcSender>().inner().clone();
    let events_manager =app.state::<EventsManager>().inner().clone();
    stable.insert(key.clone(), WorkersTableManager::new(path, ipc_sender,events_manager,worker_count.unwrap()));
    CommandStatus {
        status: true,
        message: Some(format!("worker {} started", key)),
    }
}
#[tauri::command]
async fn stop_engine<R: Runtime>(app: tauri::AppHandle<R>, key: Option<String>) -> CommandStatus {
    let kref = match key {
        None => "default".to_string(),
        Some(keyref) => keyref,
    };
    let worker_table = APPLICATION_CONTEXT.get::<WorkersTable>();
    let mut stable = worker_table.lock().await;
    if !stable.contains_key(&kref) {
        return CommandStatus {
            status: false,
            message: Some(format!("worker {} not found", kref)),
        };
    }
    match stable.remove(&kref) {
        None => CommandStatus {
            status: false,
            message: Some(format!("worker {} not found", kref)),
        },
        Some(main_worker_stable) => {
            drop(main_worker_stable);
            CommandStatus {
                status: true,
                message: Some(format!("worker {} stoped", kref)),
            }
        }
    }
}
#[tauri::command]
async fn restart_engine<R: Runtime>(app: tauri::AppHandle<R>, key: Option<String>, worker_count: Option<usize>) -> CommandStatus {

    let kref: String = match key {
        None => "main".to_string(),
        Some(keyref) => keyref,
    };
    let worker_table = APPLICATION_CONTEXT.get::<WorkersTable>();
    let mut stable = worker_table.lock().await;
    if !stable.contains_key(&kref) {
        return CommandStatus {
            status: false,
            message: Some(format!("worker {} not found", kref)),
        };
    }
    match stable.remove(&kref) {
        None => CommandStatus {
            status: false,
            message: Some(format!("worker {} not found", kref)),
        },
        Some(main_worker_stable) => {
            let ipc_sender =app.state::<IpcSender>().inner().clone();
            let events_manager =app.state::<EventsManager>().inner().clone();
            stable.insert(kref.clone(), WorkersTableManager::new(main_worker_stable.main_nodule.clone(),ipc_sender, events_manager,worker_count.unwrap()));
            let _ = app.emit_all("runtimeRestart", ());
            drop(main_worker_stable);
            CommandStatus {
                status: true,
                message: Some(format!("worker {} stoped", kref)),
            }
        }
    }
}

#[tauri::command]
async fn request<R: Runtime>(app: tauri::AppHandle<R>, name: String, content: String) {
    let ipc_sender =app.state::<IpcSender>().inner().clone();
    let _ = ipc_sender.send(IpcMessage::SentToDeno(name, content)).await.unwrap();
    println!("request success");
}

async fn run<R: Runtime>(handle_ref: tauri::AppHandle<R>) {
    let ipc_recever =handle_ref.state::<IpcReceiver>().inner().clone();
    let events_manager =handle_ref.state::<EventsManager>().inner().clone();
    println!("run");
    loop {
        match ipc_recever.recv().await.unwrap() {
            IpcMessage::SentToWindow(msg) => {
                let window = handle_ref.get_window(&msg.id);
                match window {
                    Some(window) => {
                        let _ = window.emit_all(&msg.event, msg.content);
                    },
                    None => {
                        let _ = handle_ref.emit_all(&msg.event, msg.content);
                    },
                }
                
            },
            IpcMessage::SentToDeno(name, content) => {
                events_manager
                .send(name, content.clone())
                .await
                .unwrap();
            },
        }
    }
}


pub struct DenoServer<R: Runtime> {
    main_module: String,
    invoke_handler: Box<dyn Fn(Invoke<R>) + Send + Sync>,
    events_manager: EventsManager,
    pub deno_sender: IpcSender,
    pub deno_receiver: IpcReceiver,
}
impl<R: Runtime> DenoServer<R> {
    pub fn new( main_module: String) -> Self {
    let (deno_sender,deno_receiver) =async_channel::unbounded::<IpcMessage>();
      Self {
        invoke_handler: Box::new(tauri::generate_handler![restart_engine, stop_engine, start_engine,request]),
        main_module,
        events_manager: EventsManager::new(),
        deno_sender,
        deno_receiver
      }
    }
}

impl<R: Runtime> Plugin<R> for DenoServer<R> {
    fn name(&self) -> &'static str {
        "ipcs"
    }
    fn initialization_script(&self) -> Option<String> {
        None
      }
    fn initialize(&mut self, _app: &AppHandle<R>, _config: serde_json::Value) -> PluginResult<()> {
        _app.manage(self.deno_sender.clone());
        _app.manage(self.deno_receiver.clone());
        _app.manage(self.events_manager.clone());
        let handle_ref: AppHandle<R> = _app.clone();
        let mut map = HashMap::new();
        map.insert("main".to_string(), WorkersTableManager::new(self.main_module.clone(), self.deno_sender.clone(),self.events_manager.clone(),1));
        let workers_table: Mutex<HashMap<String, WorkersTableManager>> = WorkersTable::new(map);
        APPLICATION_CONTEXT.set(workers_table);
        println!("initialize");
        tokio::task::spawn(run(handle_ref));
        Ok(())
      }
    fn created(&mut self, _window: Window<R>) {   
    }
    fn on_page_load(&mut self, _window: Window<R>, _payload: PageLoadPayload) {}
      
    fn on_event(&mut self, _app: &tauri::AppHandle<R>, _event: &tauri::RunEvent) {}
    fn extend_api(&mut self, invoke: Invoke<R>) {
        println!("extend_api");
        (self.invoke_handler)(invoke)
    }
}
