use crate::config::config::ApplicationConfig;
use crate::config::option::Opt;
use crate::{AppContext, ApplicationContext};
use clap::Parser;
use tokio::fs::read_to_string;

pub async fn init_config() {
    ApplicationContext::set_service(set_config().await);
}

async fn set_config() -> ApplicationConfig {
    let opt = Opt::parse();
    let content = match opt.config_path.as_str() {
        "" => {
            let mut path = "./bootstrap.toml";
            #[cfg(debug_assertions)]{
                path = "./src-tauri/bootstrap.toml";
            }

            read_to_string(path).await.unwrap()
        }
        _ => read_to_string(opt.config_path.clone()).await.unwrap(),
    };
    ApplicationConfig::from_toml(&content)
}
