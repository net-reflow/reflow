use failure::Error;
use std::sync::Arc;

pub mod listen;
mod inspect;
pub mod route;
pub mod forwarding;
pub mod conf;

pub use self::route::TcpRouter;
use self::listen::listen_socks;
pub use self::conf::AppConf;
use std::path::Path;
use resolver::create_resolver;
use conf::load_reflow_rules;
use conf::{DomainMatcher,IpMatcher};
use futures_cpupool::CpuPool;

pub fn run_with_conf<'a>(conf: &AppConf,
                         path: &'a Path,
                         d: Arc<DomainMatcher>,
                         i: Arc<IpMatcher>,
                         p: CpuPool,
) -> Result<(), Error> {
    let resolver = Arc::new(create_resolver(conf.relay.resolver));
    let tcp_rules = load_reflow_rules(&path.join("tcp.reflow"), &conf.get_gateways())?;
    let router = TcpRouter::new(d, i, tcp_rules);
    listen_socks(&conf.relay.listen_socks, resolver, Arc::new(router), p)?;
    Ok(())
}