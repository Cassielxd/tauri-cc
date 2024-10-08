use deno_lib::args::flags_from_vec;
use deno_lib::deno_ipc::{events_manager::EventsManager, IpcSender};
use deno_lib::deno_runtime::deno_core::v8;
use deno_lib::deno_runtime::tokio_util::create_and_run_current_thread;
use deno_lib::deno_runtime::WorkerExecutionMode;
use deno_lib::factory::CliFactory;
use deno_lib::tools::run::maybe_npm_install;
use deno_lib::util;
use deno_lib::util::file_watcher::WatcherRestartMode;
use futures::task::AtomicWaker;
use serde::Deserialize;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::thread;
use tokio::select;

macro_rules! svec {
  ($($x:expr),* $(,)?) => (vec![$($x.to_string().into()),*]);
}
#[derive(Clone)]
pub struct WorkerManager {
  pub main_nodule: String,
  pub worker_handle: Option<MainWorkerHandle>,
  pub events_manager: EventsManager,
}

impl WorkerManager {
  pub fn new(key: String, main_path: String, deno_sender: IpcSender) -> WorkerManager {
    #[cfg(not(debug_assertions))]
    {
      WorkerManager::run(key,main_path,deno_sender)
    }
    #[cfg(debug_assertions)]
    {
      WorkerManager::run_with_watch(key,main_path,deno_sender)
    }
  }
  pub fn run_with_watch(key: String, main_path: String, deno_sender: IpcSender) -> WorkerManager {
    let events_manager = EventsManager::new();
    let events_manager_ref = events_manager.clone();
    let main_path_ref = main_path.clone();
    let build = thread::Builder::new().name(format!("deno-engine-{}", key));
    let _ = build.spawn(move || {
      let args = svec!["", "run", "--allow-all", main_path.as_str()];
      // 将args转换为flagset
      let flags = Arc::new(flags_from_vec(args).unwrap());
      let future = util::file_watcher::watch_recv_ipc(
        flags,
        deno_sender.clone(),
        events_manager.clone(),
        util::file_watcher::PrintConfig::new_with_banner("Watcher", "Process", true),
        WatcherRestartMode::Automatic,
        move |flags, deno_sender_ref, events_manager_ref, watcher_communicator, _changed_paths| {
          Ok(async move {
            let factory = CliFactory::from_flags_for_watcher(flags, watcher_communicator.clone());
            let cli_options = factory.cli_options()?;
            let main_module = cli_options.resolve_main_module()?;

            maybe_npm_install(&factory).await?;
            factory.ipc_state_resolver_new(deno_sender_ref, events_manager_ref).await;
            let _ = watcher_communicator.watch_paths(cli_options.watch_paths());

            let worker = factory.create_cli_main_worker_factory().await?.create_main_worker(WorkerExecutionMode::Run, main_module.clone()).await?;
            worker.run_for_watcher().await?;
            Ok(())
          })
        },
      );
      let _ = create_and_run_current_thread(future);
    });
    WorkerManager {
      worker_handle: None,
      main_nodule: main_path_ref,
      events_manager: events_manager_ref,
    }
  }
  pub fn run(key: String, main_path: String, deno_sender: IpcSender) -> WorkerManager {
    let events_manager = EventsManager::new();
    let events_manager_ref = events_manager.clone();
    let main_path_ref = main_path.clone();
    // 创建一个用于线程间通信的同步通道
    let (handle_sender, handle_receiver) = sync_channel::<MainWorkerHandle>(1);
    // 创建一个线程，并为其命名
    let build = thread::Builder::new().name(format!("deno-engine-{}", key));
    // 隐藏的线程任务，用于执行JavaScript引擎的初始化和运行"resource/main.ts".into()
    let _ = build.spawn(move || {
      let args = svec!["", "run", "--allow-all", main_path.as_str()];
      // 将args转换为flagset
      let flags = Arc::new(flags_from_vec(args).unwrap());

      let future = async {
        let factory = CliFactory::from_flags(flags);
        let cli_options = factory.cli_options().unwrap();
        // 解析主模块
        let main_module = cli_options.resolve_main_module().unwrap();
        // 运行npm install
        maybe_npm_install(&factory).await.unwrap();
        factory.ipc_state_resolver_new(deno_sender, events_manager).await;
        // 创建CLI主工作线程工厂实例
        let worker_factory = factory.create_cli_main_worker_factory().await.unwrap();

        // 创建自定义工作线程实例
        let mut main_worker: deno_lib::worker::CliMainWorker = worker_factory.create_main_worker(WorkerExecutionMode::Run, main_module.clone()).await.unwrap();
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
    let worker_handle: MainWorkerHandle = handle_receiver.recv().unwrap();
    // 创建MainWorkerThread实例
    WorkerManager {
      worker_handle: worker_handle.into(),
      main_nodule: main_path_ref,
      events_manager: events_manager_ref,
    }
  }
}

impl Drop for WorkerManager {
  fn drop(&mut self) {
    if let Some(worker_handle) = self.worker_handle.clone() {
      worker_handle.clone().terminate();
      worker_handle.sender.send_blocking(1).expect("error");
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

#[derive(Serialize, Deserialize)]
pub struct CommandStatus {
  pub status: bool,
  pub message: Option<String>,
}
