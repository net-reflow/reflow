use std::net::SocketAddr;
use std::net::IpAddr;
use std::collections::BTreeMap;
use failure::Error;

use super::Egress;
use bytes::Bytes;
use std::fmt::Debug;
use super::decision_tree::{RoutingBranch};
use core::fmt;
use std::path::Path;
use std::fs;

mod parse;

use self::parse::{conf_items, Item};
use std::sync::Arc;
use conf;
use std::mem;

pub struct MainConf {
    pub dns: Option<DnsProxy>,
    pub relays: Vec<Relay>,
    pub domain_matcher: Arc<conf::DomainMatcher>,
    pub ip_matcher: Arc<conf::IpMatcher>,
}

pub fn load_conf(p: &Path)-> Result<MainConf, Error> {
    let f = fs::read(&p.join("config"))?;
    let (_, is) = conf_items(&f)
        .map_err(|e| format_err!("error parsing config: {:?}", e))?;
    let mut rules = BTreeMap::new();
    let mut dns = None;
    let mut relays = vec![];
    let mut egresses = BTreeMap::new();
    for it in is {
        match it {
            Item::Rule(r) => {
                rules.insert(r.name, r.branch);
            },
            Item::Dns(x) => dns = Some(x),
            Item::Relay(x) => relays.push(x),
            Item::Egress(x) => {
                egresses.insert(x.name.clone(), x);
            },
        };
    }
    let ip_matcher = Arc::new(conf::IpMatcher::new(p)?);
    let d = Arc::new(conf::DomainMatcher::new(p)?);
    Ok(MainConf {
        dns: dns,
        relays: relays,
        domain_matcher: d,
        ip_matcher: ip_matcher,
    })
}

#[derive(Clone, Debug)]
pub struct NameServer {
    pub egress: Option<RefVal<Egress>>,
    pub remote: NameServerRemote,
}
#[derive(Clone, Debug)]
pub enum NameServerRemote{
    Udp(SocketAddr),
    Tcp(SocketAddr),
}
impl NameServerRemote {
    pub fn sock_addr(&self) -> SocketAddr {
        match self {
            NameServerRemote::Udp(a) => *a,
            NameServerRemote::Tcp(a) => *a,
        }
    }
}

pub struct Relay {
    resolver: Option<NameServer>,
    pub listen: RelayProto,
    pub rule: RefVal<RoutingBranch>,
}
impl fmt::Debug for Relay {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Relay on {:?}", self.listen)?;
        Ok(())
    }
}
#[derive(Debug)]
pub enum RelayProto {
    Socks5(SocketAddr),
}
#[derive(Debug)]
pub struct DnsProxy {
    pub listen: SocketAddr,
    pub forward: BTreeMap<Bytes, NameServer>,
    pub default: NameServer,
}

pub struct Rule {
    name: Bytes,
    branch: RoutingBranch,
}
#[derive(Clone, Debug)]
pub enum RefVal<T>{
    Ref(Bytes),
    Val(T),
}

impl<T: Clone> RefVal<T> {
    pub fn get_ref(&self)-> Option<Bytes> {
        match self {
            RefVal::Ref(x) => Some(x.clone()),
            _ => None,
        }
    }
    fn insert_value(&mut self, valmap: &BTreeMap<Bytes, T>)->Result<(), Error> {
        if let Some(n) = self.get_ref() {
            let g = valmap.get(&n)
                .ok_or_else(|| format_err!("Variable {:?} is not defined", n))?;
            mem::replace(self, RefVal::Val(g.clone()));
        }
        Ok(())
    }
}
impl<T> RefVal<T> {
    pub fn val(&self)->&T {
        match self {
            RefVal::Val(v) => &v,
            RefVal::Ref(n) => panic!("{:?} is not value", n),
        }
    }
}

impl Relay {
    pub fn nameserver_or_default(&self)-> NameServer {
        match self.resolver {
            Some(ref r) => r.clone(),
            None => {
                NameServer {
                    egress: None,
                    remote: NameServerRemote::Udp(
                        SocketAddr::new(
                            IpAddr::from([8,8,8,8]),
                            53)),
                }
            }
        }
    }
}
impl DnsProxy {
    fn new1(listen: SocketAddr, ms: Vec<(Bytes,NameServer)>)->Result<DnsProxy, Error> {
        let mut forward: BTreeMap<Bytes, NameServer> = ms.into_iter().collect();
        let b: Bytes = "else".into();
        let d = forward.remove(&b)
            .ok_or_else(||format_err!("No default dns"))?;
        Ok(DnsProxy {
            listen: listen,
            forward: forward,
            default:d,
        })
    }

    /// replace named gateways with actual values
    fn deref_route(mut self, gw: &BTreeMap<Bytes, Egress>)
                   -> Result<DnsProxy, Error> {
        let f = self.forward.into_iter().map(|(k, mut v)| {
            if let Some(ref mut e) = v.egress {
                e.insert_value(gw).unwrap();
            }
            (k, v)
        }).collect();
        if let Some(ref mut e) = self.default.egress {
            e.insert_value(gw)?;
        }
        Ok(DnsProxy {
            listen: self.listen,
            forward: f,
            default: self.default,
        })
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