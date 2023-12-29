use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use micro_engine::args::{flags_from_vec};
use micro_engine::deno_runtime::permissions::PermissionsContainer;
use micro_engine::deno_runtime::tokio_util::create_and_run_current_thread;
use micro_engine::factory::CliFactory;
use micro_engine::tools::run::maybe_npm_install;
use std::thread;
use futures::task::AtomicWaker;
use tokio::select;
use micro_engine::deno_runtime::deno_core::{CancelHandle, v8};
use micro_engine::deno_runtime::deno_core::error::AnyError;

pub type MainWorkersTable = HashMap<String, MainWorkerThread>;
lazy_static! {
  pub static ref MAIN_WORKER_STABLE: Mutex<MainWorkersTable> =Mutex::new(HashMap::new());
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
        let schedule_termination =
            !self.termination_signal.swap(true, Ordering::SeqCst);
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
    ctrl_closed: bool,
    message_closed: bool,
}

impl Drop for MainWorkerThread {
    fn drop(&mut self) {
        self.worker_handle.clone().terminate();
        self.worker_handle.clone().sender.send_blocking(1).expect("error");
    }
}

/*
REQUEST_CHANNEL主要用于 web 默认路由不存在
向jsruntime 发送request使用
*/

//初始化脚本引擎
pub fn init_engine(key: String) -> Result<(), AnyError> {
    // 创建一个用于线程间通信的同步通道
    let (handle_sender, handle_receiver) = std::sync::mpsc::sync_channel::<
        Result<MainWorkerHandle, AnyError>,
    >(1);
    // 创建一个线程，并为其命名
    let build = thread::Builder::new().name(format!("js-engine"));
    // 隐藏的线程任务，用于执行JavaScript引擎的初始化和运行
    let _ = build.spawn(|| {
        let args = vec!["".into(), "run".into(), "resource/main.ts".into()];
        // 将args转换为flagset
        let flags = flags_from_vec(args).unwrap();
        let future = async {
            let factory = CliFactory::from_flags(flags).await.unwrap();
            let cli_options = factory.cli_options();
            // 解析主模块
            let main_module = cli_options.resolve_main_module().unwrap();
            // 运行npm install
            maybe_npm_install(&factory).await.unwrap();
            // 创建CLI主工作线程工厂实例
            let worker_factory = factory.create_cli_main_worker_factory().await.unwrap();
            // 创建自定义工作线程实例
            let mut worker = worker_factory
                .create_custom_worker(main_module, PermissionsContainer::allow_all(), vec![simple_http::simple_http::init_ops_and_esm()], Default::default())
                .await
                .unwrap();
            // 获取工作线程的JavaScript运行时线程安全句柄
            let handle = worker.worker.js_runtime.v8_isolate().thread_safe_handle();
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
            handle_sender.send(Ok(external_handle)).unwrap();
            drop(handle_sender);
            // 选择执行不同的分支 有一个返回线程结束
            select! {
        res = receiver.recv() => {
          println!("结束了{:?}",res);
        }
        code = worker.run() => {
          println!("run {:?}",code);
         }
      }
        };
        // 创建并运行当前线程
        create_and_run_current_thread(future);
    });
    // 获取handle_receiver通道接收到的值，即MainWorkerHandle实例
    let worker_handle = handle_receiver.recv().unwrap().unwrap();
    // 创建MainWorkerThread实例
    let worker_thread = MainWorkerThread {
        worker_handle: worker_handle.into(),
        ctrl_closed: false,
        message_closed: false,
    };
    let mut stable = MAIN_WORKER_STABLE.lock().unwrap();
    stable.insert(key, worker_thread);
    Ok(())
}

#[allow(dead_code)]
pub fn stop_engine() {
    let mut stable = MAIN_WORKER_STABLE.lock().unwrap();
    if let Some(worker_thread) = stable.remove(&*"default".to_string()) {
        drop(worker_thread);
    } else {
        println!("default engine not found")
    }
}

