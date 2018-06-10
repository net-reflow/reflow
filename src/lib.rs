extern crate bytes;
#[macro_use]
extern crate failure;
extern crate futures;
#[macro_use]
extern crate log;
extern crate tokio;
extern crate trust_dns;
extern crate toml;
extern crate trust_dns_server;
extern crate trust_dns_proto;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate env_logger;

use std::error::Error;
use std::sync::Arc;
use futures::future;

mod resolver;
mod ruling;

pub fn run(config_path: &str)-> Result<(), Box<Error>> {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();
    // We provide a way to *instantiate* the service for each new
    // connection; here, we just immediately return a new instance.
    let ruler = ruling::DomainMatcher::new(config_path)?;
    let _ip_matcher = ruling::IpMatcher::new(config_path)?;
    let d = Arc::new(ruler);

    let ds = resolver::DnsServer::new(d.clone(), config_path)?;
    tokio::run( future::lazy(move || {
        if let Err(e) = ds.start() {
            error!("Dns server error: {:?}", e);
        }
        Ok::<_, ()>(())
    }));
    Ok(())
}
