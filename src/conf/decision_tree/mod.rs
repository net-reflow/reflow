//! the routing is configured using a tree-like structure

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::{Formatter};
use std::mem;
use failure::Error;
use bytes::Bytes;

mod text;

use relay::route::TcpTrafficInfo;
pub use self::text::var_name;
pub use self::text::read_branch;
use conf::Egress;
use conf::main::RefVal;
use util::BsDisp;

#[derive(Clone)]
pub enum RoutingBranch {
    /// try them one by one, return the first match, if there is one
    /// otherwise there is no result
    Sequential(Vec<RoutingBranch>),
    Conditional(RoutingCondition),
    /// a match is found
    Final(RoutingAction)
}


impl RoutingBranch {
    pub fn decision(&self, info: &TcpTrafficInfo)-> Option<RoutingAction> {
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

    fn new_final(x: RoutingAction)-> RoutingBranch {
        RoutingBranch::Final(x)
    }

    pub fn insert_gateways(&mut self, gw: &BTreeMap<Bytes, Egress>) -> Result<(), Error> {
        use self::RoutingBranch::*;
        match self {
            Final(ref mut d) =>{
                d.insert_gateways(gw)?
            }
            Conditional(ref mut c) => c.insert_gateways(gw)?,
            Sequential(ref mut s) => {
                for mut r in s {
                    r.insert_gateways(gw)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub enum RoutingCondition {
    Domain(BTreeMap<Bytes, RoutingBranch>),
    IpAddr(BTreeMap<Bytes, RoutingBranch>),
    Port(u16, Box<RoutingBranch>),
    Protocol(BTreeMap<Bytes, RoutingBranch>),
}

impl RoutingCondition {
    fn decide(&self, info: &TcpTrafficInfo)-> Option<RoutingAction> {
        use self::RoutingCondition::*;
        match self {
            Domain(x) => {
                let r = x.get(info.domain_region()?)?;
                r.decision(info)
            }
            IpAddr(x) => x.get(info.ip_region()?)?.decision(info),
            Port(x, y) => if info.addr().port() != *x { None } else { y.decision(info) },
            Protocol(x) => x.get(info.protocol().name())?.decision(info),
        }
    }

    fn insert_gateways(&mut self, gw: &BTreeMap<Bytes, Egress>) -> Result<(), Error> {
        use self::RoutingCondition::*;
        let m = match self {
            Port(_, ref mut b) => {
                return b.insert_gateways(gw);
            }
            Domain(x) => x,
            IpAddr(x) => x,
            Protocol(x) => x,
        };
        for (_k, mut v) in m {
            v.insert_gateways(gw)?;
        }
        Ok(())
    }
}

/// a chosen route
#[derive(Clone)]
pub enum RoutingAction {
    Direct,
    Reset,
    Named(RefVal<Egress>),
}

impl RoutingAction {
    fn new_named(x: &[u8]) -> RoutingAction {
        let g = RefVal::Ref(x.into());
        RoutingAction::Named(g)
    }

    pub fn insert_gateways(&mut self, gw: &BTreeMap<Bytes, Egress>) -> Result<(), Error> {
        match self {
            RoutingAction::Named(e) => {
                if let Some(n) = e.get_ref() {
                    let g = gw.get(&n)
                        .ok_or_else(|| format_err!("Unknown gateway {:?}", n))?;
                    mem::replace(e, RefVal::Val(g.clone()));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl fmt::Display for RoutingBranch {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
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

impl fmt::Debug for RoutingBranch {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        use self::RoutingBranch::*;
        match self {
            Sequential(x) => {
                write!(f, "any [\n")?;
                for y in x {
                    write!(f, "{:?}\n", y)?;
                }
                write!(f, "]")?;
            }
            Conditional(x) => write!(f, "cond {}", x)?,
            Final(x) => write!(f, "egress {}", x)?,
        }
        Ok(())
    }
}

impl fmt::Display for RoutingCondition {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
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

impl fmt::Display for RoutingAction {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        use self::RoutingAction::*;
        match self {
            Direct => write!(f, "direct"),
            Reset  => write!(f, "reset"),
            Named(s) => {
                let n = match s {
                    RefVal::Ref(n) => &n,
                    RefVal::Val(x) => &x.name,
                };
                write!(f, "{}", BsDisp::new(&n))
            },
        }
    }
}

fn print_mapping(map: &BTreeMap<Bytes, RoutingBranch>, f: &mut Formatter)
                 -> Result<(), fmt::Error> {
    write!(f, "{{\n")?;
    for (k,v) in map.iter() {
        write!(f, "{:?} => {}\n", k, v)?;
    }
    write!(f, "}}")?;
    Ok(())
}
