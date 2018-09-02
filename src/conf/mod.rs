mod prefix_match;
mod decision_tree;
mod main;
mod util;

pub use self::prefix_match::domain_name::DomainMatcher;
pub use self::prefix_match::ip_addr::IpMatcher;
pub use self::decision_tree::RoutingAction;
pub use self::decision_tree::RoutingBranch;
use std::net::SocketAddr;
use std::net::IpAddr;
use bytes::Bytes;
pub use self::main::{DnsProxy, NameServer, NameServerRemote};
pub use self::main::{load_conf, MainConf, Relay, RelayProto};
use std::fmt;
use util::BsDisp;

#[derive(Debug, Clone)]
pub struct Egress {
    pub name: Bytes,
    pub addr: EgressAddr,
}
#[derive(Copy, Clone, Debug)]
pub enum EgressAddr {
    Socks5(SocketAddr),
    /// Bind to an address before connecting
    From(IpAddr),
}

impl Egress {
    pub fn addr(&self)->EgressAddr {
        self.addr
    }
}

impl fmt::Display for Egress {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Egress {}", BsDisp::new(&self.name))?;
        write!(f, ": {:?}", self.addr)?;
        Ok(())
    }
}