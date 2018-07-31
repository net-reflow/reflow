use std::net::IpAddr;

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
use relay::forwarding::routing::conf::RoutingAction;

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

    pub fn route(&self, addr: SocketAddr, protocol: &TcpProtocol)-> Option<Route> {
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
        d.and_then(|x| match x.route {
            RoutingAction::Direct => Some(Route::Direct),
            RoutingAction::Reset => Some(Route::Reset),
            RoutingAction::From(i) => Some(Route::From(i)),
            RoutingAction::Named(ref x) => self.gateways.get(x)
                .map(|x| Route::Socks(SocketAddr::new(x.host, x.port))),
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Route {
    Direct,
    /// Bind to an address before connecting
    From(IpAddr),
    Reset,
    Socks(SocketAddr),
}