use getset::{Getters, Setters};

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize, Clone, Getters, Setters, Default)]
#[getset(get_mut = "pub", get = "pub", set = "pub")]
pub struct ServerConfig {
  host: String,
  port: Option<u16>,
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize, Clone, Getters, Setters, Default)]
#[getset(get_mut = "pub", get = "pub", set = "pub")]
pub struct LogConfig {
  log_dir: String,
  log_temp_size: String,
  log_pack_compress: String,
  log_rolling_type: String,
  log_level: String,
}

///服务启动配置
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize, Clone, Getters, Setters, MutGetters, Default)]
#[getset(get_mut = "pub", get = "pub", set = "pub")]
pub struct ApplicationConfig {
  debug: bool,
  ///default path "target/logs/"
  log: LogConfig,
  server: ServerConfig,
}

impl ApplicationConfig {
  pub fn from_toml(toml_data: &str) -> Self {
    let config = match toml::from_str(toml_data) {
      Ok(e) => e,
      Err(e) => panic!("{}", e),
    };
    config
  }
}
