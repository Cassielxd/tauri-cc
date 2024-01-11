// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri_desktop::init_context;

#[tokio::main]
async fn main() {
    init_context().await;
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)] //仅在调试时自动打开开发者工具
            {
                let main_window = app.get_window("main").unwrap();
                main_window.open_devtools();
            }
            Ok(())
        })
        .plugin(tauri_plugin_http_server::init(None))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
