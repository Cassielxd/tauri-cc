#[macro_use]
extern crate getset;

pub mod config;
pub mod initialize;

use crate::initialize::config::init_config;
use state::Container;

pub static APPLICATION_CONTEXT: Container![Send + Sync] = <Container![Send + Sync]>::new();

pub async fn init_context() {
  init_config().await;
}
