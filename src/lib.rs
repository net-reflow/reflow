#![feature(await_macro, async_await)]

extern crate byteorder;
extern crate bytes;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate futures01;
extern crate httparse;
#[macro_use]
extern crate log;
extern crate net2;
#[macro_use]
extern crate nom;
#[macro_use]
extern crate structopt;
extern crate env_logger;
extern crate tokio;
extern crate tokio_io;
extern crate treebitmap;
extern crate trust_dns;
extern crate trust_dns_resolver;

use structopt::StructOpt;

mod cmd_options;
mod conf;
mod relay;
mod resolver;
pub mod util;

use crate::conf::load_conf;
use crate::relay::run_with_conf;
use futures::executor::ThreadPool;
use futures::Future;
use futures::task::Context;
use std::task::Poll;
use std::pin::Pin;

pub fn run() -> Result<(), i32> {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();
    let opt = cmd_options::Opt::from_args();
    let config_path = &opt.config;
    if !config_path.is_dir() {
        error!("The given configuration directory doesn't exist");
        return Err(99);
    }

    let conf = match load_conf(&config_path) {
        Ok(x) => x,
        Err(e) => {
            error!("Error in config: {}", e);
            return Err(100);
        }
    };
    let mut pool = ThreadPool::new().map_err(|e| {
        error!("Error in ThreadPool: {}", e);
        101
    })?;
    let p1 = pool.clone();
    pool.run(async move {
        for r in conf.relays {
            info!("Starting {}", r);
            if let Err(e) = run_with_conf(
                r,
                conf.domain_matcher.clone(),
                conf.ip_matcher.clone(),
                p1.clone(),
            ) {
                error!("Relay error: {:?}", e);
            }
        }
        if let Some(dns) = conf.dns {
            info!("Starting dns proxy");
            let ds = resolver::serve(dns, conf.domain_matcher.clone(), p1);
            if let Err(e) = ds {
                error!("Dns server error: {:?}", e);
            }
        }
        Forever().await
    });
    Ok(())
}

struct Forever();
impl Future for Forever {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
        Poll::Pending
    }
}