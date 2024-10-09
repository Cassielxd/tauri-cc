// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri_desktop::config::config::ApplicationConfig;
use tauri_desktop::init_context;
use tauri_desktop::APPLICATION_CONTEXT;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  init_context().await;
  let ref_app_config = APPLICATION_CONTEXT.get::<ApplicationConfig>();
  let mut build = tauri::Builder::default(); 
  build = build.setup(|app| {
    #[cfg(debug_assertions)] //仅在调试时自动打开开发者工具
    {
      let main_window = app.get_webview_window("main").unwrap();
      main_window.open_devtools();
    }
    Ok(())
  });
  #[cfg(not(debug_assertions))]
  {
    let path = ref_app_config.pro_code_path();
    build = build.plugin(tauri_plugin_deno::init(path.into()));
  }
  #[cfg(debug_assertions)]
  {
    let path = ref_app_config.dev_code_path();
    build = build.plugin(tauri_plugin_deno::init(path.into())).plugin(tauri_plugin_devtools::init());
  }
  build = build.invoke_handler(tauri::generate_handler![sync_message, async_message]);
  build.run(tauri::generate_context!()).expect("error while running tauri application");
  Ok(())
}

//同步消息

#[tauri::command(rename_all = "snake_case")]
fn sync_message(invoke_message: String) -> Result<String, ()> {
  println!("同步调用: {}", invoke_message);
  Ok(format!("{}", invoke_message))
}

//异步消息
#[tauri::command(rename_all = "snake_case")]
async fn async_message(value: &str) -> Result<String, ()> {
  // Call another async function and wait for it to finish
  some_async_function().await;
  // Note that the return value must be wrapped in `Ok()` now.
  Ok(format!("异步调用 {}", value))
}
async fn some_async_function() {
  println!("异步调用");
}
