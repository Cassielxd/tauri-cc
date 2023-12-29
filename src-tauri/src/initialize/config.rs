use clap::Parser;
use tokio::fs::read_to_string;
use crate::{AppContext, ApplicationContext};
use crate::config::config::ApplicationConfig;
use crate::config::option::Opt;

pub async fn init_config() {
    ApplicationContext::set_service(set_config().await);
}

async fn set_config() -> ApplicationConfig {
    let opt = Opt::parse();
    let content = match opt.config_path.as_str() {
        "" => {
            read_to_string("./resource/bootstrap.toml").await.unwrap()
        }
        _ => {
            read_to_string(opt.config_path.clone()).await.unwrap()
        }
    };
    ApplicationConfig::from_toml(&content)
}
