//! parse the configuration file
use std::str;
use std::string::ToString;
use std::collections::BTreeMap;
use nom::{
    alpha,
    be_u8,be_u16, be_u24, be_u32, ErrorKind,
    Err,
    is_space,
    line_ending,
    multispace0,
    not_line_ending,
    newline,
    space0,
    space1,
};
use super::{
    RoutingAction,
    RoutingBranch,
    RoutingCondition,
    RoutingDecision
};

named!(get_reflow<&[u8], RoutingCondition>,
    do_parse!(
        tag_s!("Tree-Format: reflow 0.1") >>
        newline_maybe_space >>
        d: dbg_dmp!(read_cond ) >>
        ( d )
    )
);

named!(read_cond<&[u8], RoutingCondition>,
    do_parse!(
        tag_s!("cond") >>
        space1 >>
        kind: take_till!(is_space) >>
        space0 >>
        d: switch!(value!(kind),
            b"domain" => map!(read_mapping, |m| RoutingCondition::Domain(m)) |
            b"ip" => map!(read_mapping, |m| RoutingCondition::IpAddr(m)) |
            b"protocol" => map!(read_mapping, |m| RoutingCondition::Protocol(m))
          ) >>
        ( (d) )
    )
);

named!(read_mapping<&[u8], BTreeMap<String, RoutingBranch> >,
    do_parse!(
        char!('{') >>
        newline_maybe_space >>
        m: dbg_dmp!(read_map) >>
        //char!('}') >>
        ( m )
    )
);

named!(read_map<&[u8], BTreeMap<String, RoutingBranch> >,
    do_parse!(
        entries: separated_nonempty_list!(newline_maybe_space, read_map_entry) >>
        ( entries.into_iter().collect() )
    )
);

/// doesn't consume spaces before or after it
named!(read_map_entry<&[u8], (String, RoutingBranch)>,
    do_parse!(
        keyword: map_res!(
                     alt!( delimited!(char!('"'), take_until!("\""), char!('"')) |
                           take_till!(is_space)
                         ),
                str::from_utf8
                 ) >>
space0 >>
tag!("=>") >>
space0 >>
        value: read_branch >>
        ( (keyword.to_string(), value) )
    )
);

named!(read_branch<&[u8], RoutingBranch>,
    do_parse!(
        value: map!(read_decision, |deci| RoutingBranch::Final(deci)) >>
        ( value )
    )
);

/// leaf node of the decision tree
named!(read_decision<&[u8], RoutingDecision>,
    do_parse!(
        value: alt!(
            map!(tag!("do direct"), |_| RoutingDecision::direct()) |
            map!(tag!("do reset"), |_| RoutingDecision {route: RoutingAction::Reset, additional: vec![]}) |
            map!(map_res!(delimited!(char!('"'), take_until!("\""), char!('"')),
                          str::from_utf8),
                 |s| RoutingDecision {route: RoutingAction::named(s), additional: vec![]})
        ) >>
        ( value )
    )
);

/// consume one \n and any number of other whitespaces
named!(newline_maybe_space<&[u8], ()>,
    do_parse!(
        space0 >>
        line_ending >>
        multispace0 >>
        ( () )
    )
);

#[cfg(test)]
mod tests{
    use std::fs::read_to_string;
    use super::*;
    #[test]
    fn test() {
        let conf = read_to_string("config/tcp.reflow").unwrap();
        let g = get_reflow(conf.as_bytes());
        match g {
            Ok(k) =>println!("okay {:?}", k),
            Err(x) => println!("err {:?}", x),
        };
    }
}