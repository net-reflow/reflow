//! parse the configuration file

use nom::{be_u8,be_u16, be_u24, be_u32, ErrorKind};
use super::{RoutingCondition};

named!(get_reflow<&str, Vec<&str> >,
    do_parse!(
        tag_s!("Tree-Format: reflow 0.1\n") >>
        d: get_domain >>
        ( d )
    )
);

named!(get_domain<&str, Vec<&str> >,
    do_parse!(
        tag_s!("domain") >>
        ws!(char!('{')) >>
        m: read_map >>
        //char!('}') >>
        ( m )
    )
);

named!(read_map<&str, Vec<&str> >,
do_parse!(
ent: read_map_entry>>
tag_s!("\n") >>
//ent1: read_map_entry>>
(vec![ent])
)
//separated_nonempty_list!(ws!(char!('\n')), read_map_entry)
);

named!(read_map_entry<&str, &str>,
do_parse!(
keyword: delimited!(char!('"'), take_until!("\""), char!('"')) >>
ws!(tag!("=>")) >>
take_until!("\n") >>
( keyword )
)
);

#[cfg(test)]
mod tests{
    use std::fs::read_to_string;
    use super::*;
    #[test]
    fn test() {
        let conf = read_to_string("config/tcp.reflow").unwrap();
        let g = get_reflow(&conf);
        println!("{:?}", g);
    }
}