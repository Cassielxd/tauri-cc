// Prevents additional console window on Windows in release, DO NOT REMOVE!!
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri_desktop::init_context;

#[tokio::main]
async fn main() {
    init_context().await;
    let mut build = tauri::Builder::default();
    build = build.setup(|app| {
        #[cfg(debug_assertions)] //仅在调试时自动打开开发者工具
        {
            let main_window = app.get_window("main").unwrap();
            main_window.open_devtools();
        }
        Ok(())
    });
    let mut path = "./resource/main.ts";
    #[cfg(debug_assertions)]
    {
        path = "./src-tauri/resource/main.ts";
    }
    build = build.plugin(tauri_plugin_deno::init(None, path.into()));
    build.run(tauri::generate_context!())
        .expect("error while running tauri application");
}
