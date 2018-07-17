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
};
use super::{RoutingCondition, RoutingAction};

named!(get_reflow<&[u8], BTreeMap<String, RoutingAction> >,
    do_parse!(
        tag_s!("Tree-Format: reflow 0.1") >>
        newline_maybe_space >>
        d: dbg_dmp!(get_domain ) >>
        ( d )
    )
);

named!(read_map<&[u8], BTreeMap<String, RoutingAction> > ,
    do_parse!(
        entries: separated_nonempty_list!(newline_maybe_space, read_map_entry) >>
        ( entries.into_iter().collect() )
    )
);

named!(get_domain<&[u8], BTreeMap<String, RoutingAction> >,
    do_parse!(
        tag_s!("cond domain") >>
        space0 >>
        char!('{') >>
        newline_maybe_space >>
        m: dbg_dmp!(read_map) >>
        //char!('}') >>
        ( m )
    )
);

named!(read_map_entry<&[u8], (String, RoutingAction)>,
    do_parse!(
        keyword: map_res!(delimited!(char!('"'), take_until!("\""), char!('"')),
                str::from_utf8
                 ) >>
space0 >>
tag!("=>") >>
space0 >>
        value: alt!(
            map!(tag!("direct"), |_| RoutingAction::Direct) |
            map!(tag!("reset"), |_| RoutingAction::Reset) |
            map!(map_res!(delimited!(char!('"'), take_until!("\""), char!('"')),
                          str::from_utf8),
                 RoutingAction::named)
        ) >>
        space0 >>
        ( (keyword.to_string(), value) )
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