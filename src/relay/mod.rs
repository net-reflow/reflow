use failure::Error;
use std::sync::Arc;

pub mod listen;
mod inspect;
pub mod route;
pub mod forwarding;

pub use self::route::TcpRouter;
use self::listen::listen_socks;
use crate::resolver::create_resolver;
use crate::conf::{DomainMatcher,IpMatcher};
use crate::conf::Relay;
use crate::conf::RelayProto;

/// Start a relay
pub fn run_with_conf(conf: Relay,
                     d: Arc<DomainMatcher>,
                     i: Arc<IpMatcher>,
) -> Result<(), Error> {
    let rule = conf.rule.val().clone();
    // FIXME support proxy
    let ns = conf.nameserver_or_default();
    if !ns.egress.is_none() {
        error!("Resolver config in relay doesn't support proxy yet");
    }
    let resolver = Arc::new(create_resolver(
        ns.remote)
    );
    let router = TcpRouter::new(d, i, rule);
    match conf.listen {
        RelayProto::Socks5(a) => {
            listen_socks(&a, resolver, Arc::new(router))?;
        }
    }
    Ok(())
}