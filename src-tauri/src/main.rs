// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::borrow::BorrowMut;

use tauri::Manager;
use tauri_desktop::{init_context};
use tauri_desktop::config::config::ApplicationConfig;
use tauri_desktop::APPLICATION_CONTEXT;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_context().await;
    let ref_app_config = APPLICATION_CONTEXT.get::<ApplicationConfig>();
    let mut build = tauri::Builder::default();
    build = build.setup(|app| {
        #[cfg(debug_assertions)] //仅在调试时自动打开开发者工具
        {
            let main_window = app.get_window("main").unwrap();
            main_window.open_devtools();
        }
        Ok(())
    });
    #[cfg(not(debug_assertions))]
    {
        let path = ref_app_config.pro_code_path();
        build = build.plugin(tauri_plugin_deno::DenoServer::new(path.into()));
    }
    #[cfg(debug_assertions)]
    {
        let path = ref_app_config.dev_code_path();
        build = build.plugin(tauri_plugin_deno::DenoServer::new(path.into()));
    }

    build.run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}
