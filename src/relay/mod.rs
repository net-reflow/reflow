use failure::Error;
use std::sync::Arc;

pub mod incoming;
mod upstream;
pub mod forwarding;
mod conf;

use self::incoming::listen_socks;
use self::conf::RelayConf;
use std::path::Path;
use resolver::create_resolver;

pub fn run_with_conf(path: &Path)-> Result<(), Error> {
    let conf = RelayConf::new(path)?;
    let resolver = Arc::new(create_resolver(conf.resolver));
    listen_socks(&conf.listen_socks, resolver)?;
    Ok(())
}