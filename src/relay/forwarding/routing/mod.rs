mod conf;

pub use self::conf::RoutingDecision;
pub use self::conf::load_reflow_rules;
use std::sync::Arc;
use std::net::SocketAddr;

use ruling::{DomainMatcher, IpMatcher};
use relay::forwarding::routing::conf::RoutingBranch;
use relay::forwarding::tcp::TcpProtocol;

struct TcpTrafficInfo<'a> {
    addr: SocketAddr,
    protocol: TcpProtocol<'a>,
    pub domain_region: Option<&'a str>,
    ip_region: Option<&'a str>,
}

pub struct TcpRouter {
    domain_match: Arc<DomainMatcher>,
    ip_match: Arc<IpMatcher>,
    rules: RoutingBranch,
}

impl TcpRouter {
    pub fn new(domain_match: Arc<DomainMatcher>,
               ip_match: Arc<IpMatcher>,
               rules: RoutingBranch,
    ) -> TcpRouter {
        TcpRouter { domain_match, ip_match, rules }
    }

    pub fn route(&self, addr: SocketAddr, protocol: TcpProtocol)-> Option<RoutingDecision> {
        let i = TcpTrafficInfo { addr, protocol, domain_region: None, ip_region: None };
        let d = self.rules.decision(&i);
        // let d = d.map(|x| x.clone());
        d
    }
}
