use failure::Error;
use std::sync::Arc;

pub mod incoming;
mod upstream;
pub mod forwarding;
mod conf;

pub use self::forwarding::TcpRouter;
use self::incoming::listen_socks;
use self::conf::RelayConf;
use std::path::Path;
use resolver::create_resolver;
use relay::forwarding::load_reflow_rules;
use ruling::{DomainMatcher,IpMatcher};

pub fn run_with_conf(path: &Path, d: Arc<DomainMatcher>, i: Arc<IpMatcher>)-> Result<(), Error> {
    let conf = RelayConf::new(path)?;
    let resolver = Arc::new(create_resolver(conf.resolver));
    let tcp_rules = load_reflow_rules(&path.join("tcp.reflow"))?;
    let router = TcpRouter::new(d, i, tcp_rules);
    listen_socks(&conf.listen_socks, resolver, Arc::new(router))?;
    Ok(())
}