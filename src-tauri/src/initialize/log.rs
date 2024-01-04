use crate::config::config::ApplicationConfig;
use crate::{AppContext, ApplicationContext};
use fast_log::config::Config;
use fast_log::consts::LogSize;
use fast_log::plugin::file_split::RollingType;
use fast_log::plugin::packer::ZipPacker;
use std::time::Duration;

/**
 *description:初始化日志配置
 *author:cassie-lxd<348040933@qq.com>
 */
pub fn init_log() {
  let c = ApplicationContext::get_service::<ApplicationConfig>();
  if let Some(cof) = c {
    let cassie_config = cof.lock().unwrap();
    let log_config = cassie_config.log();
    //create log dir
    std::fs::create_dir_all(log_config.log_dir()).unwrap();
    //initialize fast log
    fast_log::init(
      Config::new()
        .console()
        .file_split(log_config.log_dir(), str_to_temp_size(log_config.log_temp_size()), str_to_rolling(log_config.log_rolling_type()), ZipPacker {})
        .level(str_to_log_level(log_config.log_level())),
    )
    .unwrap();
  }
}

fn str_to_temp_size(arg: &str) -> LogSize {
  match arg {
    arg if arg.ends_with("MB") => {
      let end = arg.find("MB").unwrap();
      let num = arg[0..end].to_string();
      LogSize::MB(num.parse::<usize>().unwrap())
    }
    arg if arg.ends_with("KB") => {
      let end = arg.find("KB").unwrap();
      let num = arg[0..end].to_string();
      LogSize::KB(num.parse::<usize>().unwrap())
    }
    arg if arg.ends_with("GB") => {
      let end = arg.find("GB").unwrap();
      let num = arg[0..end].to_string();
      LogSize::GB(num.parse::<usize>().unwrap())
    }
    _ => LogSize::MB(100),
  }
}

fn str_to_rolling(arg: &str) -> RollingType {
  match arg {
    arg if arg.starts_with("KeepNum(") => {
      let end = arg.find(")").unwrap();
      let num = arg["KeepNum(".len()..end].to_string();
      RollingType::KeepNum(num.parse::<i64>().unwrap())
    }
    arg if arg.starts_with("KeepTime(") => {
      let end = arg.find(")").unwrap();
      let num = arg["KeepTime(".len()..end].to_string();
      RollingType::KeepTime(Duration::from_secs(num.parse::<u64>().unwrap()))
    }
    _ => RollingType::All,
  }
}

fn str_to_log_level(arg: &str) -> log::LevelFilter {
  return match arg {
    "warn" => log::LevelFilter::Warn,
    "error" => log::LevelFilter::Error,
    "trace" => log::LevelFilter::Trace,
    "info" => log::LevelFilter::Info,
    "debug" => log::LevelFilter::Debug,
    _ => log::LevelFilter::Info,
  };
}
