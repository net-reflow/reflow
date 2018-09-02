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
use util::BsDisp;
use super::util::all_comments_or_space;

pub struct MainConf {
    pub dns: Option<DnsProxy>,
    pub relays: Vec<Relay>,
    pub domain_matcher: Arc<conf::DomainMatcher>,
    pub ip_matcher: Arc<conf::IpMatcher>,
}

pub fn load_conf<P: AsRef<Path>>(p: P)-> Result<MainConf, Error> {
    let p = p.as_ref();
    let f = fs::read(p.join("config"))?;
    let (remain, is) = conf_items(&f)
        .map_err(|e| format_err!("error parsing config: {:?}", e))?;
    if !all_comments_or_space(remain) {
        if let Some(l)= is.last() {
            error!("Only some of the configurations are successfully parsed, the last one is {}", l);
        } else {
            error!("Error parsing config, nothing is successfully parsed");
        }
        let r: Vec<u8> = remain.iter().take(20).map(|x| *x).collect();
        error!("Failed to parse the rest of the config, starting with: \"{}\"...", BsDisp::new(&r));
    }
    let mut rules = BTreeMap::new();
    let mut dns = None;
    let mut relays = vec![];
    let mut egresses = BTreeMap::new();
    for it in is {
        match it {
            Item::Rule(r) => {
                rules.insert(r.name, r.branch);
            },
            Item::Dns(x) => {
                info!("Loaded dns proxy configuration: {}", x);
                dns = Some(x);
            },
            Item::Relay(x) => {
                info!("Loaded relay configuration: {:?}", x);
                relays.push(x);
            },
            Item::Egress(x) => {
                egresses.insert(x.name.clone(), x);
            },
        };
    }
     if dns.is_none() && relays.len() == 0 {
         error!("Loaded no dns proxy configuration or relay configuration, \
          the program won't serve anything");
     }
    {
        let ks: Vec<&Bytes> = rules.keys().collect();
        check_var_name(ks)?;
        let es: Vec<&Bytes> = egresses.keys().collect();
        check_var_name(es)?;
    }
    for (_name, mut rule) in &mut rules {
        rule.insert_gateways(&egresses)?;
    }
    for mut relay in &mut relays {
        relay.rule.insert_value(&rules).map_err(|e| {
            format_err!("Rule {} is not defined", BsDisp::new(&e))
        })?;
        if let Some(ref mut res) = relay.resolver {
            if let Some(ref mut e) = res.egress {
                e.insert_value(&egresses).map_err(|e| {
                    format_err!("Egress {} is not defined", BsDisp::new(&e))
                })?;
            }
        }
    }
    if let Some(ref mut d ) = dns {
        d.deref_route(&egresses)?;
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
impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Rule {}", BsDisp::new(&self.name))?;
        Ok(())
    }
}

impl fmt::Debug for MainConf {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if let Some(ref x) = self.dns {
            writeln!(f, "dns proxy {:?}", x)?;
        }
        for x in &self.relays {
            writeln!(f, "{:?}", x)?;
        }
        Ok(())
    }
}

fn check_var_name(ns: Vec<&Bytes>)->Result<(), Error> {
    let reserved = vec![
        "bind", "else", "socks5",
        "any", "cond",
    ];
    for n in ns {
        for r in &reserved {
            if n == r.as_bytes() {
                return Err(format_err!("{} not allowed as variable name to avoid conflicts",
                         BsDisp::new(n)
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::load_conf;
    #[test]
    fn test() {
        let conf = load_conf("config");
        match conf {
            Ok(k) =>println!("okay {:?}", k),
            Err(x) => println!("err {:?}", x),
        };
    }
}