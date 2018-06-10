extern crate bytes;
#[macro_use]
extern crate failure;
extern crate futures;
#[macro_use]
extern crate log;
#[macro_use] extern crate structopt;
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
use structopt::StructOpt;

mod resolver;
mod ruling;
mod cmd_options;

pub fn run()-> Result<(), Box<Error>> {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();
    let opt = cmd_options::Opt::from_args();
    let config_path = &opt.config;
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
