mod prefix_match;
mod decision_tree;
mod main;
mod util;

pub use self::prefix_match::domain_name::DomainMatcher;
pub use self::prefix_match::ip_addr::IpMatcher;
pub use self::decision_tree::RoutingAction;
pub use self::decision_tree::Gateway;
pub use self::decision_tree::RoutingBranch;
pub use self::decision_tree::RoutingDecision;
pub use self::decision_tree::load_reflow_rules;
use std::net::SocketAddr;
use std::net::IpAddr;
use bytes::Bytes;

#[derive(Debug)]
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
