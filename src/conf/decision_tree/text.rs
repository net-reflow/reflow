//! parse the configuration file
use super::super::util::{line_sep, opt_line_sep};
use super::{RoutingAction, RoutingBranch, RoutingCondition};
use bytes::Bytes;
use nom::{digit1, line_ending, multispace0, space0, space1};
use std::collections::BTreeMap;
use std::str;

named!(read_cond<&[u8], RoutingCondition>,
    do_parse!(
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

named!(read_mapping<&[u8], BTreeMap<Bytes, RoutingBranch> >,
    do_parse!(
        char!('{') >> opt_line_sep >>
        m: dbg_dmp!(read_map) >>
        opt_line_sep >> char!('}') >>
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

named!(read_map<&[u8], BTreeMap<Bytes, RoutingBranch> >,
    do_parse!(
        entries: separated_nonempty_list!(line_sep, read_map_entry) >>
        ( entries.into_iter().collect() )
    )
);

/// doesn't consume spaces before or after it
named!(read_map_entry<&[u8], (Bytes, RoutingBranch)>,
    do_parse!(
        keyword: map!(var_name, |bs: &[u8]| bs.into()) >>
space0 >>
tag!("=>") >>
space0 >>
        value: read_branch >>
        ( (keyword, value) )
    )
);

named!(pub read_branch<&[u8], RoutingBranch>,
    switch!(terminated!(var_name, space0),
        b"direct" => value!(RoutingBranch::new_final(RoutingAction::Direct)) |
        b"reset" => value!(RoutingBranch::new_final(RoutingAction::Reset)) |
        b"any" => delimited!(tag!("["), read_sequential, tag!("]")) |
        b"cond" => map!(read_cond, |c| RoutingBranch::Conditional(c)) |
        x => value!(RoutingBranch::new_final(RoutingAction::new_named(x)))
    )
);

named!(read_sequential<&[u8], RoutingBranch>,
    do_parse!(
        opt_line_sep >>
        items:   separated_nonempty_list!(line_sep,
                     read_branch
                 ) >>
        opt_line_sep >>
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

named!(pub var_name<&[u8], &[u8]>,
    take_while!( is_alphanumunder )
);

fn is_alphanumunder(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-'
}
