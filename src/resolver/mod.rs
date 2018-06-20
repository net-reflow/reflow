//! Proxy dns requests based on rules

mod handler;
pub mod client;
mod dnsclient;
#[allow(dead_code)]
mod monitor_failure;
pub mod config;
mod serve;

pub use self::serve::serve;
