#![feature(try_from)]
#![feature(rust_2018_preview)]

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
extern crate net2;
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
extern crate core;

use failure::Error;
use futures::future;
use futures::Future;
use structopt::StructOpt;
use futures_cpupool::CpuPool;
use tokio::runtime::Runtime;

mod proto;
mod relay;
mod resolver;
mod conf;
mod cmd_options;
pub mod util;

use relay::run_with_conf;
use conf::load_conf;

pub fn run()-> Result<(), Error> {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();
    let opt = cmd_options::Opt::from_args();
    let config_path = &opt.config;
    if !config_path.is_dir() {
        return Err(format_err!("The given configuration directory doesn't exist"));
    }
    let pool = CpuPool::new_num_cpus();

    let conf = load_conf(&config_path)?;
    let mut rt = Runtime::new()?;
    rt.spawn( future::lazy(move || {
        for r in conf.relays {
            info!("Starting {:?}", r);
            if let Err(e) = run_with_conf(r,
                                          conf.domain_matcher.clone(),
                                          conf.ip_matcher.clone(),
                                          pool.clone()) {
                error!("Relay error: {:?}", e);
            }
        }

        if let Some(dns) = conf.dns {
            info!("Starting dns proxy");
            let ds = resolver::serve(dns, conf.domain_matcher.clone(), pool);
            if let Err(e) = ds {
                error!("Dns server error: {:?}", e);
            }
        }
        Ok::<_, ()>(())
    }));
    rt.shutdown_on_idle().wait().expect("Can't wait tokio runtime");
    Ok(())
}
