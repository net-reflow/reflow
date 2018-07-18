//! the routing is configured using a tree-like structure

use std::collections::BTreeMap;

mod text;

use super::super::tcp::TcpTrafficInfo;

#[derive(Debug)]
pub enum RoutingBranch {
    /// try them one by one, return the first match, if there is one
    /// otherwise there is no result
    Sequential(Vec<RoutingBranch>),
    Conditional(RoutingCondition),
    /// a match is found
    Final(RoutingDecision)
}

impl RoutingBranch {
    pub fn decision(&self, info: &TcpTrafficInfo)-> Option<&RoutingDecision> {
        use self::RoutingBranch::*;
        match self {
            Final(d) => Some(d),
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
}

#[derive(Debug)]
pub enum RoutingCondition {
    Domain(BTreeMap<String, RoutingBranch>),
    IpAddr(BTreeMap<String, RoutingBranch>),
    Port(u16, Box<RoutingBranch>),
    Protocol(BTreeMap<String, RoutingBranch>),
}

impl RoutingCondition {
    fn decide(&self, info: &TcpTrafficInfo)-> Option<&RoutingDecision> {
        use self::RoutingCondition::*;
        match self {
            Domain(x) => {
                let d = info.get_domain()?;
                let d = d.as_ref();
                let r = x.get(d)?;
                r.decision(info)
            }
            _ => unimplemented!()
        }
    }
}

#[derive(Debug)]
pub struct RoutingDecision {
    route: RoutingAction,
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
#[derive(Debug)]
enum RoutingAction {
    Direct,
    Reset,
    Named(String)
}

impl RoutingAction {
    fn named(name: &str) -> RoutingAction {
        RoutingAction::Named(name.to_string())
    }
}

#[derive(Debug)]
enum AdditionalAction {
    PrintLog,
    SaveSample,
}
