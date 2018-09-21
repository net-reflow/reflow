//! the routing is configured using a tree-like structure

use std::collections::BTreeMap;
mod text;

use super::super::tcp::TcpTrafficInfo;

enum RoutingBranch {
    /// try them one by one, return the first match, if there is one
    /// otherwise there is no result
    Sequential(Vec<RoutingBranch>),
    Conditional(RoutingCondition),
    /// a match is found
    Final(RoutingDecision)
}

impl RoutingBranch {

    pub fn decision(&self, info: &TcpTrafficInfo)-> Option<RoutingDecision> {
        use self::RoutingBranch::*;
        match self {
            Final(d) => Some(d),
            Conditional(c) => c,
            Sequential(s) => {}
        }
    }
}

enum RoutingCondition {
    Domain(BTreeMap<String, RoutingBranch>),
    IpAddr(BTreeMap<String, RoutingBranch>),
    Port(u16),
    Protocol(BTreeMap<String, RoutingBranch>),
}

impl RoutingCondition {
    fn decide(&self, info: &TcpTrafficInfo)-> Option<RoutingDecision> {
    }
}


struct  RoutingDecision {
    route: RoutingAction,
    additional: Vec<AdditionalActions>,
}

/// a chosen route
enum RoutingAction {
    Direct,
    Reset,
    Named(String)
}

enum AdditionalActions {
    PrintLog,
    SaveSample { size: usize, num: usize }
}
