use clap::Parser;
use serde::Serialize;
/**
*description:启动参数 --CASSIE_CONFIG XXX 可主动指定启动配置文件
*author:cassie-lxd<348040933@qq.com>
*/
#[derive(Debug, Clone, Parser, Serialize)]
#[clap(version)]
pub struct Opt {
  #[clap(long, env = "config", default_value = "")]
  pub config_path: String,
}
