//! Proxy dns requests based on rules

pub mod client;
mod dnsclient;
mod handler;
mod lookup;
mod serve;

pub use self::lookup::AsyncResolver;
pub use self::serve::serve;
