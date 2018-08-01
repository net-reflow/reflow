
pub mod conf;

use bytes::Bytes;
pub use self::conf::RoutingDecision;
pub use self::conf::load_reflow_rules;
pub use self::conf::Gateway;
use std::sync::Arc;
use std::net::SocketAddr;

use ruling::{DomainMatcher, IpMatcher};
use relay::forwarding::routing::conf::RoutingBranch;
use relay::forwarding::tcp::TcpProtocol;

#[derive(Debug)]
pub struct TcpTrafficInfo<'a> {
    addr: SocketAddr,
    protocol: &'a TcpProtocol,
    pub domain_region: Option<Bytes>,
    ip_region: Option<Bytes>,
}

impl<'a> TcpTrafficInfo<'a> {
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

    pub fn route(&self, addr: SocketAddr, protocol: &TcpProtocol)-> Option<RoutingDecision> {
        let domain = protocol.get_domain()
            .and_then(|x| {
                let x: Vec<&[u8]> = x.split(|&y | y==b'.').rev().collect();
                let x = x.join(&b'.');
                self.domain_match.rule_domain(&x)
            });
        let ip = self.ip_match.match_ip(addr.ip());
        let i = TcpTrafficInfo { addr, protocol: &protocol, domain_region: domain, ip_region: ip };
        let d = self.rules.decision(&i);
        info!("route {:?}, tcp traffic {:?}", d, i);
        d
    }
}