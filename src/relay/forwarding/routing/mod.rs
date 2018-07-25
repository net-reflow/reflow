mod conf;

use bytes::Bytes;
pub use self::conf::RoutingDecision;
pub use self::conf::load_reflow_rules;
use std::sync::Arc;
use std::net::SocketAddr;

use ruling::{DomainMatcher, IpMatcher};
use relay::forwarding::routing::conf::RoutingBranch;
use relay::forwarding::tcp::TcpProtocol;
use std::collections::BTreeMap;
use relay::conf::SocksConf;
use std::net::IpAddr;
use relay::forwarding::routing::conf::RoutingAction;

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
    gateways: BTreeMap<Bytes, SocksConf>,
}

impl TcpRouter {
    pub fn new(domain_match: Arc<DomainMatcher>,
               ip_match: Arc<IpMatcher>,
               rules: RoutingBranch,
               gateways: BTreeMap<Bytes, SocksConf>,
    ) -> TcpRouter {
        TcpRouter { domain_match, ip_match, rules, gateways }
    }

    pub fn route(&self, addr: SocketAddr, protocol: TcpProtocol)-> Option<Route> {
        let domain = protocol.get_domain()
            .and_then(|x| self.domain_match.rule_domain(x));
        let ip = self.ip_match.match_ip(addr.ip());
        let i = TcpTrafficInfo { addr, protocol, domain_region: None, ip_region: ip };
        let d = self.rules.decision(&i);
        d.and_then(|x| match x.route {
            RoutingAction::Direct => Some(Route::Direct),
            RoutingAction::Reset => Some(Route::Reset),
            RoutingAction::Named(ref x) => self.gateways.get(x)
                .map(|x| Route::Socks(x.clone())),
        })
    }
}

pub enum Route {
    Direct,
    Reset,
    Socks(SocksConf),
}