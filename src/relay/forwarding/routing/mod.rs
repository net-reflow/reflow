mod conf;

use bytes::Bytes;
pub use self::conf::RoutingDecision;
pub use self::conf::load_reflow_rules;
use std::sync::Arc;
use std::net::SocketAddr;

use ruling::{DomainMatcher, IpMatcher};
use relay::forwarding::routing::conf::RoutingBranch;
use relay::forwarding::tcp::TcpProtocol;

struct TcpTrafficInfo {
    addr: SocketAddr,
    protocol: TcpProtocol,
    pub domain_region: Option<Bytes>,
    ip_region: Option<Bytes>,
}

impl TcpTrafficInfo {
    pub fn domain_region(&self)-> Option<&[u8]> {
        if let Some(ref x) = self.domain_region { Some(x) } else { None }
    }

    pub fn ip_region(&self) -> Option<&[u8]> {
        if let Some(ref x) = self.ip_region { Some(x) } else { None }
    }
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

    pub fn route(&self, addr: SocketAddr, protocol: TcpProtocol)-> Option<Arc<RoutingDecision>> {
        let domain = protocol.get_domain()
            .and_then(|x| self.domain_match.rule_domain(x));
        let i = TcpTrafficInfo { addr, protocol, domain_region: None, ip_region: None };
        let d = self.rules.decision(&i);
        // let d = d.map(|x| x.clone());
        d
    }
}
