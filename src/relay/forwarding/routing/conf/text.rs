//! parse the configuration file
use std::str;
use std::string::ToString;
use std::collections::BTreeMap;
use nom::{
    alpha,
    digit1,
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
    AdditionalAction,
    RoutingAction,
    RoutingBranch,
    RoutingCondition,
    RoutingDecision
};

named!(get_reflow<&[u8], RoutingBranch>,
    do_parse!(
        tag_s!("Tree-Format: reflow 0.1") >>

        v: many1!(
            preceded!(newline_maybe_space,
                alt!(map!(read_decision, |deci| RoutingBranch::Final(deci)) |
                           map!(read_cond, |c| RoutingBranch::Conditional(c))))) >>
        ( RoutingBranch::Sequential(v) )
    )
);

named!(read_cond<&[u8], RoutingCondition>,
    do_parse!(
        tag_s!("cond") >>
        space1 >>
        kind: var_name >>
        space0 >>
        d: switch!(value!(kind),
            b"domain" => map!(read_mapping, |m| RoutingCondition::Domain(m)) |
            b"ip" => map!(read_mapping, |m| RoutingCondition::IpAddr(m)) |
            b"protocol" => map!(read_mapping, |m| RoutingCondition::Protocol(m)) |
            b"port" => call!(read_port)
          ) >>
        ( (d) )
    )
);

named!(read_mapping<&[u8], BTreeMap<String, RoutingBranch> >,
    do_parse!(
        char!('{') >>
        multispace0 >>
        m: dbg_dmp!(read_map) >>
        multispace0 >>
        char!('}') >>
        ( m )
    )
);

named!(read_port<&[u8], RoutingCondition>,
    do_parse!(
        // space already consumed in read_cond
        tag!("eq")>>
        space1 >>
        port: read_u16 >>
        space0 >>
        tag!("=>")>>
        space0 >>
        branch: read_branch >>
        ( RoutingCondition::Port(port, Box::new(branch)) )
    )
);

named!(read_u16<&[u8], u16>,
    map_res!(map_res!(digit1, str::from_utf8),
             str::FromStr::from_str)
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
        keyword: map_res!(var_name,
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
    alt!(map!(read_decision, |deci| RoutingBranch::Final(deci)) |
         preceded!(tuple!(tag!("any"), space0), delimited!(tag!("["), read_sequential, tag!("]"))) |
         map!(read_cond, |c| RoutingBranch::Conditional(c))
        )
);

/// leaf node of the decision tree
named!(read_decision<&[u8], RoutingDecision>,
    do_parse!(
        route: alt!(
            switch!(preceded!(pair!(tag!("do"), space1), var_name),
                        b"direct" => value!(RoutingAction::Direct) |
                        b"reset" => value!(RoutingAction::Reset)
                   ) |
            map!(map_res!(preceded!(pair!(tag!("use"), space1), var_name),
                          str::from_utf8),
                 |s| RoutingAction::named(s))
        ) >>
        acts: many0!(read_additional_action) >>
        ( RoutingDecision {route, additional: acts } )
    )
);

named!(read_additional_action<&[u8], AdditionalAction>,
    do_parse!(
        space1 >>
        tag!("and") >>
        space1 >>
        act: alt!(map!(tag!("print_log"), |_| AdditionalAction::PrintLog) |
                  map!(tag!("save_sample"), |_| AdditionalAction::SaveSample)
             ) >>
        ( act )
    )
);

named!(read_sequential<&[u8], RoutingBranch>,
    do_parse!(
        multispace0 >>
        items:   separated_nonempty_list!(newline_maybe_space,
                      alt!(map!(read_decision, |deci| RoutingBranch::Final(deci)) |
                           map!(read_cond, |c| RoutingBranch::Conditional(c))
                          )
                 ) >>
        multispace0 >>
        ( RoutingBranch::Sequential(items) )
    )
);

/// consume one \n and any number of other whitespaces
named!(newline_maybe_space<&[u8], ()>,
    complete!(do_parse!(
        space0 >>
        line_ending >>
        multispace0 >>
        ( () )
    ))
);

named!(var_name<&[u8], &[u8]>,
    take_while!( is_alphanumunder )
);

fn is_alphanumunder(c: u8)-> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

#[cfg(test)]
mod tests{
    use std::fs::read_to_string;
    use super::*;
    #[test]
    fn test() {
        let conf = read_to_string("config/tcp.reflow").unwrap();
        let g = get_reflow(conf.as_bytes());
        match g {
            Ok((_, k)) =>println!("okay {}", k),
            Err(x) => println!("err {:?}", x),
        };
    }
}