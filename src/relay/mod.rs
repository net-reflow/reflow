use failure::Error;
use std::sync::Arc;

pub mod listen;
mod inspect;
pub mod route;
pub mod forwarding;

pub use self::route::TcpRouter;
use self::listen::listen_socks;
use resolver::create_resolver;
use conf::{DomainMatcher,IpMatcher};
use futures_cpupool::CpuPool;
use conf::Relay;
use conf::RelayProto;

/// Start a relay
pub fn run_with_conf(conf: Relay,
                     d: Arc<DomainMatcher>,
                     i: Arc<IpMatcher>,
                     p: CpuPool,
) -> Result<(), Error> {
    let rule = conf.rule.val().clone();
    // FIXME support proxy
    let resolver = Arc::new(create_resolver(
        conf.nameserver_or_default().remote.sock_addr())
    );
    let router = TcpRouter::new(d, i, rule);
    match conf.listen {
        RelayProto::Socks5(a) => {
            listen_socks(&a, resolver, Arc::new(router), p)?;
        }
    }
    Ok(())
}