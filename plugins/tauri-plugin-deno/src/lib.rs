use axum::Router;
use std::net::SocketAddr;
use std::str::FromStr;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

use futures::task::AtomicWaker;
use deno::args::flags_from_vec;
use deno::deno_runtime::deno_core::v8;
use deno::deno_runtime::WorkerExecutionMode;
use deno::deno_runtime::deno_permissions::PermissionsContainer;
use deno::deno_runtime::tokio_util::create_and_run_current_thread;
use deno::factory::CliFactory;
use deno::tools::run::maybe_npm_install;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use tokio::{select, time};
use deno::deno_fake_http::{HttpReceiver, HttpSender, RequestContext};
use state::Container;
use tokio::sync::{mpsc, RwLock};

pub static APPLICATION_CONTEXT: Container![Send + Sync] = <Container![Send + Sync]>::new();
type WorkersTable = RwLock<HashMap<String, WorkersTableManager>>;
#[derive(Clone)]
pub struct WorkersTableManager {
    pub main_worker_thread: Arc<Mutex<MainWorkerThread>>,
    pub request_channel: (HttpSender, HttpReceiver),
    pub main_nodule: String,
}

impl WorkersTableManager {
    pub fn restart(self) {
        let mut stable = self.main_worker_thread.lock().unwrap();
        *stable = MainWorkerThread::new(self.main_nodule.clone(), self.request_channel.1.clone());
    }
    pub fn new(main_nodule: String) -> Self {
        let request_channel = async_channel::unbounded::<RequestContext>();
        WorkersTableManager {
            main_worker_thread: Arc::new(Mutex::new(MainWorkerThread::new(main_nodule.clone(), request_channel.1.clone()))),
            main_nodule,
            request_channel,
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
    isolate_handle: v8::IsolateHandle,   // JavaScript Isolate 的句柄
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
    fn new(main_path: String, recever: HttpReceiver) -> MainWorkerThread {
        // 创建一个用于线程间通信的同步通道
        let (handle_sender, handle_receiver) = sync_channel::<MainWorkerHandle>(1);
        // 创建一个线程，并为其命名
        let build = thread::Builder::new().name(format!("js-engine"));
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
                let worker_factory = factory.create_cli_main_worker_factory_tauri(Some(recever)).await.unwrap();
                // 创建自定义工作线程实例
                let mut main_worker = worker_factory
                    .create_main_worker(WorkerExecutionMode::Run, main_module, PermissionsContainer::allow_all())
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

#[tauri::command]
async fn restart_engine<R: Runtime>(app: tauri::AppHandle<R>) {
    let main_worker_stable = APPLICATION_CONTEXT.get::<WorkersTableManager>();
    let mut stable = main_worker_stable.main_worker_thread.lock().unwrap();
    *stable = MainWorkerThread::new(main_worker_stable.main_nodule.clone(), main_worker_stable.request_channel.1.clone());
    let _ = app.emit_all("runtimeRestart", ());
}

async fn run<R: Runtime>(handle_ref: tauri::AppHandle<R>, addr: Option<SocketAddr>) {
    let address = match addr {
        Some(a) => a,
        None => SocketAddr::from_str("127.0.0.1:20004").unwrap()
    };
    println!(" - Local:   http://{}", address.clone());
    let app = Router::new().fallback(default_router).with_state(handle_ref);
    axum::Server::bind(&address).serve(app.into_make_service()).await.unwrap();
}

pub async fn default_router(request: Request<Body>) -> Response<Body> {
    let worker_table = APPLICATION_CONTEXT.get::<WorkersTable>();
    let sender = worker_table.read().await.get("main").unwrap().request_channel.0.clone();
    let (_response_tx, mut response_rx) = mpsc::channel(1);
    let _ = sender.send(RequestContext { request, response_tx: _response_tx.clone() }).await;
    let sleep = time::sleep(Duration::from_secs(5));
    tokio::pin!(sleep);
    select! {
      _ = &mut sleep => {
        let mut res = Response::new(Body::from("operation timed out".to_string()));
        *res.status_mut() = StatusCode::REQUEST_TIMEOUT;
        res
      }
      Some(res) = response_rx.recv() => {
         res
      }
  }
}


/// Initializes the plugin.
pub fn init<R: Runtime>(addr: Option<SocketAddr>, main_module: String) -> TauriPlugin<R> {
    Builder::new("http-server")
        .invoke_handler(tauri::generate_handler![restart_engine])
        .setup(move |handle| {
            let handle_ref = handle.clone();
            //resource/main.ts
            let mut map = HashMap::new();
            map.insert("main".to_string(), WorkersTableManager::new(main_module.clone()));
            let workers_table = WorkersTable::new(map);
            APPLICATION_CONTEXT.set(workers_table);
            tokio::task::spawn(run(handle_ref, addr));
            Ok(())
        })
        .build()
}
