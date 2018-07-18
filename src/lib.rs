#![feature(try_from)]

extern crate byteorder;
extern crate bytes;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate futures;
extern crate futures_cpupool;
extern crate httparse;
#[macro_use]
extern crate log;
#[macro_use]
extern crate nom;
extern crate socks;
#[macro_use] extern crate structopt;
extern crate tokio;
#[macro_use]
extern crate tokio_io;
extern crate treebitmap;
extern crate trust_dns;
extern crate trust_dns_resolver;
extern crate toml;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate env_logger;

use std::error::Error;
use std::sync::Arc;
use futures::future;
use futures::Future;
use structopt::StructOpt;
use futures_cpupool::CpuPool;
use tokio::runtime::Runtime;

mod proto;
mod relay;
mod resolver;
mod ruling;
mod cmd_options;

use resolver::config::DnsProxyConf;
use relay::run_with_conf;
use relay::TcpRouter;

pub fn run()-> Result<(), Box<Error>> {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();
    let opt = cmd_options::Opt::from_args();
    let config_path = &opt.config;
    let conf1 = config_path.clone();
    let pool = CpuPool::new_num_cpus();

    let ip_matcher = Arc::new(ruling::IpMatcher::new(config_path)?);
    let d = Arc::new(ruling::DomainMatcher::new(config_path)?);

    let conf = DnsProxyConf::new(&config_path.join("resolve.config"))?;
    let mut rt = Runtime::new()?;
    rt.spawn( future::lazy(move || {
        if let Err(e) = run_with_conf(&conf1, d.clone(), ip_matcher) {
            error!("Relay error: {:?}", e);
        }
        let ds = resolver::serve(d.clone(), conf, pool);
        if let Err(e) = ds {
            error!("Dns server error: {:?}", e);
        }
        Ok::<_, ()>(())
    }));
    rt.shutdown_on_idle().wait().expect("Can't wait tokio runtime");
    Ok(())
}
