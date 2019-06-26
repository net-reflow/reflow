use bytes::Bytes;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::conf::RoutingAction;
use crate::conf::RoutingBranch;
use crate::conf::{DomainMatcher, IpMatcher};
use crate::relay::inspect::TcpProtocol;
use crate::util::BsDisp;
use std::fmt;

#[derive(Debug)]
pub struct TcpTrafficInfo<'a> {
    addr: SocketAddr,
    protocol: &'a TcpProtocol,
    pub domain_region: Option<Bytes>,
    ip_region: Option<Bytes>,
}

impl<'a> TcpTrafficInfo<'a> {
    pub fn domain_region(&self) -> Option<&[u8]> {
        if let Some(ref x) = self.domain_region {
            Some(x)
        } else {
            None
        }
    }

    pub fn ip_region(&self) -> Option<&[u8]> {
        if let Some(ref x) = self.ip_region {
            Some(x)
        } else {
            None
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn protocol(&self) -> &TcpProtocol {
        self.protocol
    }
}

pub struct TcpRouter {
    domain_match: Arc<DomainMatcher>,
    ip_match: Arc<IpMatcher>,
    rules: RoutingBranch,
}

impl TcpRouter {
    pub fn new(
        domain_match: Arc<DomainMatcher>,
        ip_match: Arc<IpMatcher>,
        rules: RoutingBranch,
    ) -> TcpRouter {
        TcpRouter {
            domain_match,
            ip_match,
            rules,
        }
    }

    pub fn route(&self, addr: SocketAddr, protocol: &TcpProtocol) -> Option<RoutingAction> {
        let domain = protocol.get_domain().and_then(|x| {
            let x: Vec<&[u8]> = x.split(|&y| y == b'.').rev().collect();
            let x = x.join(&b'.');
            self.domain_match.rule_domain(&x)
        });
        let ip = self.ip_match.match_ip(addr.ip());
        let i = TcpTrafficInfo {
            addr,
            protocol: &protocol,
            domain_region: domain,
            ip_region: ip,
        };
        let d = self.rules.decision(&i);
        info!("{}", RouteAndTraffic::new(&d, i));
        d
    }
}

struct RouteAndTraffic<'a> {
    traffic: TcpTrafficInfo<'a>,
    route: &'a Option<RoutingAction>,
}

impl<'a> RouteAndTraffic<'a> {
    fn new(d: &'a Option<RoutingAction>, t: TcpTrafficInfo<'a>) -> RouteAndTraffic<'a> {
        RouteAndTraffic {
            route: d,
            traffic: t,
        }
    }
}

impl<'a> fmt::Display for RouteAndTraffic<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.route {
            Some(r) => write!(f, "Route {} ", r)?,
            None => write!(f, "Missing route! ")?,
        }
        write!(f, "{}", self.traffic)?;
        Ok(())
    }
}

impl<'a> fmt::Display for TcpTrafficInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{} ", BsDisp::new(self.protocol.name()))?;
        if let Some(d) = self.protocol.get_domain() {
            write!(f, "{}", BsDisp::new(&d))?;
            if let Some(ref r) = self.domain_region {
                write!(f, "({})", BsDisp::new(&r))?;
            }
        }
        write!(f, " addr={}", self.addr)?;
        if let Some(ref r) = self.ip_region {
            write!(f, "({})", BsDisp::new(&r))?;
        }
        match self.protocol {
            TcpProtocol::PlainHttp(h) => {
                if let Some(ref u) = h.user_agent {
                    write!(f, " ua={}", BsDisp::new(&u))?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
