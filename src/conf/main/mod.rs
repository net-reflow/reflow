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
mod util;
mod dns;
mod relay;

use self::parse::{conf_items, Item};
use std::sync::Arc;
use conf;
pub use self::util::RefVal;
pub use conf::main::dns::{DnsProxy, NameServer, NameServerRemote};
pub use conf::main::relay::{Relay, RelayProto};

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

pub struct Rule {
    name: Bytes,
    branch: RoutingBranch,
}

impl Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Rule {:?}", self.name)?;
        write!(f, ": {}", self.branch)?;
        Ok(())
    }
}