//! Proxy dns requests based on rules

mod handler;
pub mod client;
mod dnsclient;
pub mod config;
mod serve;
mod lookup;

pub use self::serve::serve;
