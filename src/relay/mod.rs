use failure::Error;
use std::sync::Arc;

pub mod forwarding;
mod inspect;
pub mod listen;
pub mod route;

use self::listen::listen_socks;
pub use self::route::TcpRouter;
use crate::conf::Relay;
use crate::conf::RelayProto;
use crate::conf::{DomainMatcher, IpMatcher};
use crate::resolver::create_resolver;
use futures::executor::ThreadPool;

/// Start a relay
pub fn run_with_conf(
    conf: Relay,
    d: Arc<DomainMatcher>,
    i: Arc<IpMatcher>,
    pool: ThreadPool,
) -> Result<(), Error> {
    let rule = conf.rule.val().clone();
    // FIXME support proxy
    let ns = conf.nameserver_or_default();
    if ns.egress.is_some() {
        error!("Resolver config in relay doesn't support proxy yet");
    }
    let resolver = Arc::new(create_resolver(ns.remote, pool.clone()));
    let router = TcpRouter::new(d, i, rule);
    match conf.listen {
        RelayProto::Socks5(a) => {
            listen_socks(&a, resolver, Arc::new(router), pool)?;
        }
    }
    Ok(())
}
