use std::net::SocketAddr;
use std::net::IpAddr;
use std::collections::BTreeMap;

use super::Egress;
use bytes::Bytes;
use std::fmt::Debug;
use super::decision_tree::{read_branch, RoutingBranch};
use core::fmt;

mod parse;

#[derive(Debug)]
struct NameServer {
    egress: Option<RefVal<Egress>>,
    remote: NameServerRemote,
}
#[derive(Debug)]
enum NameServerRemote{
    Udp(SocketAddr),
    Tcp(SocketAddr),
}
#[derive(Debug)]
struct Relay {
    resolver: Option<NameServer>,
    listen: RelayProto,
    rule: Bytes,
}
#[derive(Debug)]
enum RelayProto {
    Socks5(SocketAddr),
}
#[derive(Debug)]
struct DnsProxy {
    listen: SocketAddr,
    forward: BTreeMap<Bytes, NameServer>,
}

struct Rule {
    name: Bytes,
    branch: RoutingBranch,
}
#[derive(Debug)]
enum RefVal<T>{
    Ref(Bytes),
    Val(T),
}

impl<T: Debug> RefVal<T> {
    pub fn val(&self)->&T {
        match self {
            RefVal::Val(v) => &v,
            _ => panic!("{:?} is not value", self),
        }
    }
}

impl NameServerRemote {
    fn new(proto: &str, addr: SocketAddr)->NameServerRemote {
        match proto {
            "tcp" => NameServerRemote::Tcp(addr),
            "udp" => NameServerRemote::Udp(addr),
            _ => panic!("{} is not known DNS protocol", proto),
        }
    }
}
impl Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Rule {:?}", self.name)?;
        write!(f, ": {}", self.branch)?;
        Ok(())
    }
}