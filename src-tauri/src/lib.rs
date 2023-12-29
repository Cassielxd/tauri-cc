#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate getset;

pub mod initialize;
pub mod plugins;
pub mod config;

use std::sync::{Arc, Mutex};
use state::Container;
use crate::initialize::config::init_config;
use crate::initialize::log::init_log;


pub(crate) static APPLICATION_CONTEXT: Container![Send + Sync] = <Container![Send + Sync]>::new();


pub trait AppContext {
    fn get_service<T: 'static>() -> Option<Arc<Mutex<T>>>;
    fn set_service<T: Send + Sync + 'static>(service: T);
}

#[derive(Clone, Default)]
pub struct ApplicationContext {}

impl AppContext for ApplicationContext {
    fn get_service<T: 'static>() -> Option<Arc<Mutex<T>>> {
        let service = APPLICATION_CONTEXT.try_get::<Arc<Mutex<T>>>();
        match service {
            None => None,
            Some(s) => Some(Arc::clone(s)),
        }
    }
    fn set_service<T: Send + Sync + 'static>(ser: T) {
        let mutex = Arc::new(Mutex::new(ser));
        APPLICATION_CONTEXT.set(mutex);
    }
}

pub async fn init_context() {
    init_config().await;
    init_log();
}