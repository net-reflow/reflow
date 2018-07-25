//! the routing is configured using a tree-like structure

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::{Formatter, Error};
use std::path::Path;
use std::fs;
use failure::Error as FailureError;
use bytes::Bytes;

mod text;

use self::text::get_reflow;
use relay::forwarding::routing::TcpTrafficInfo;
use std::sync::Arc;

#[allow(dead_code)]
pub fn load_reflow_rules(p: &Path)-> Result<RoutingBranch, FailureError> {
    let bs = fs::read(p)?;
    let (_, x) = get_reflow(&bs)
        .map_err(|_| format_err!("error parsing tcp.reflow"))?;
    Ok(x)
}

pub enum RoutingBranch {
    /// try them one by one, return the first match, if there is one
    /// otherwise there is no result
    Sequential(Vec<RoutingBranch>),
    Conditional(RoutingCondition),
    /// a match is found
    Final(Arc<RoutingDecision>)
}


impl RoutingBranch {
    pub fn decision(&self, info: &TcpTrafficInfo)-> Option<Arc<RoutingDecision>> {
        use self::RoutingBranch::*;
        match self {
            Final(d) => Some(d.clone()),
            Conditional(c) => c.decide(info),
            Sequential(s) => {
                for r in s {
                    if let Some(d) = r.decision(info) {
                        return Some(d)
                    }
                }
                None
            }
        }
    }

    fn new_final(x: RoutingDecision)-> RoutingBranch {
        RoutingBranch::Final(Arc::new(x))
    }
}

pub enum RoutingCondition {
    Domain(BTreeMap<Bytes, RoutingBranch>),
    IpAddr(BTreeMap<Bytes, RoutingBranch>),
    Port(u16, Box<RoutingBranch>),
    Protocol(BTreeMap<Bytes, RoutingBranch>),
}

impl RoutingCondition {
    fn decide(&self, info: &TcpTrafficInfo)-> Option<Arc<RoutingDecision>> {
        use self::RoutingCondition::*;
        match self {
            Domain(x) => {
                let r = x.get(info.domain_region()?)?;
                r.decision(info)
            }
            IpAddr(x) => x.get(info.ip_region()?)?.decision(info),
            Port(x, y) => if info.addr.port() != *x { None } else { y.decision(info) },
            Protocol(x) => x.get(info.protocol.name())?.decision(info),
        }
    }
}

pub struct RoutingDecision {
    pub route: RoutingAction,
    additional: Vec<AdditionalAction>,
}

impl RoutingDecision {
    /// simplest
    pub fn direct()-> RoutingDecision {
        RoutingDecision {
            route: RoutingAction::Direct,
            additional: vec![],
        }
    }
}


/// a chosen route
pub enum RoutingAction {
    Direct,
    Reset,
    Named(Bytes)
}

enum AdditionalAction {
    PrintLog,
    SaveSample,
}

impl fmt::Display for RoutingBranch {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        use self::RoutingBranch::*;
        match self {
            Sequential(x) => {
                write!(f, "[\n")?;
                for y in x {
                    write!(f, "{}\n", y)?;
                }
                write!(f, "]")?;
            }
            Conditional(x) => write!(f, "cond {}", x)?,
            Final(x) => write!(f, "{}", x)?,
        }
        Ok(())
    }
}

impl fmt::Display for RoutingCondition {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        use self::RoutingCondition::*;
        match self {
            Domain(ref m) => { write!(f, "domain ")?; print_mapping(m, f)?; }
            IpAddr(ref m) => { write!(f, "ip ")?; print_mapping(m, f)?;}
            Protocol(ref m) => { write!(f, "protocol ")?; print_mapping(m, f)?;}
            Port(x, y) => { write!(f, "port eq {} => {}", x, y)?; }
        }
        Ok(())
    }
}

impl fmt::Display for RoutingDecision {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.route)?;
        for i in self.additional.iter() {
            write!(f, " and {}", i)?;
        }
        Ok(())
    }
}

impl fmt::Display for RoutingAction {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        use self::RoutingAction::*;
        match self {
            Direct => write!(f, "do direct"),
            Reset  => write!(f, "do reset"),
            Named(s) => write!(f, "use {:?}", s),
        }
    }
}

impl fmt::Display for AdditionalAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::AdditionalAction::*;
        match self {
            PrintLog => write!(f, "print_log"),
            SaveSample => write!(f, "save_sample"),
        }
    }
}

fn print_mapping(map: &BTreeMap<Bytes, RoutingBranch>, f: &mut Formatter)
                 -> Result<(), Error> {
    write!(f, "{{\n")?;
    for (k,v) in map.iter() {
        write!(f, "{:?} => {}\n", k, v)?;
    }
    write!(f, "}}")?;
    Ok(())
}
