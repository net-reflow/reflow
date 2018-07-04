use failure::Error;

pub mod incoming;
mod upstream;
mod forwarding;
mod conf;

use self::incoming::listen_socks;
use self::conf::RelayConf;
use std::path::Path;

pub fn run_with_conf(path: &Path)-> Result<(), Error> {
    let conf = RelayConf::new(path)?;
    listen_socks(&conf.listen_socks)?;
    Ok(())
}