use std::net::SocketAddr;
use std::str::FromStr;
use axum::Router;

use crate::{AppContext, ApplicationContext};
use crate::config::config::ApplicationConfig;


use tauri::{AppHandle, plugin::{Builder, TauriPlugin}, RunEvent, Runtime};
use simple_http::default_router;
use crate::initialize::micro_engine::init_engine;

async fn run_http() {
    let mut addr = SocketAddr::from_str("127.0.0.1:20003").unwrap();
    let c = ApplicationContext::get_service::<ApplicationConfig>();
    if let Some(cof) = c {
        let cassie_config = cof.lock().unwrap();
        let server = cassie_config.server();
        addr = SocketAddr::from_str(&format!("{}:{}", server.host(), server.port().unwrap_or(20003))).unwrap();
    }
    println!(" - Local:   http://{}", addr.clone());
    let app = Router::new().fallback(default_router);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}


async fn start<R: Runtime>(_app_handler: AppHandle<R>) {
    run_http().await;
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("server")
        .setup(|handle| {
            let _ = init_engine("default".to_string());
            tokio::task::spawn(start(handle.clone()));
            Ok(())
        })
        .build()
}