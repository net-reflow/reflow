use std::str;
use std::fmt;
use nom::{space0, space1};
use std::net::{SocketAddr, IpAddr};
use super::super::decision_tree::var_name;
use bytes::Bytes;
use super::Egress;
use super::super::EgressAddr;
use super::super::util::{line_sep, opt_line_sep};
use super::super::decision_tree::{read_branch};
use super::{NameServer, NameServerRemote, RefVal, Relay, RelayProto, DnsProxy, Rule};
use std::fmt::Formatter;

pub enum Item {
    Egress(Egress),
    Relay(Relay),
    Dns(DnsProxy),
    Rule(Rule),
}

named!(pub conf_items<&[u8], Vec<Item>>,
    preceded!(
      opt_line_sep,
      separated_list_complete!(line_sep, conf_item)
   )
);
named!(conf_item<&[u8], Item>,
    do_parse!(
        kind: var_name >>
        space0 >>
        d: switch!(value!(kind),
            b"egress" => map!(read_egress, |m| Item::Egress(m)) |
            b"relay" => map!(relay_conf, |x| Item::Relay(x)) |
            b"dns" => map!(dns_conf, |x| Item::Dns(x)) |
            b"rule" => map!(rule_conf, |x| Item::Rule(x))
          ) >>
        ( d )
    )
);

named!(read_egress<&[u8], Egress>,
    do_parse!(
        name: var_name >>
        equals >>
        d: alt!(egress_socks5|egress_interface) >>
        ( Egress{name: name.into(), addr: d} )
    )
);

named!(equals<&[u8], ()>,
    map!(tuple!(space0, tag!("="), space0), |_| ())
);
named!(egress_socks5<&[u8], EgressAddr>,
   do_parse!(
        tag_s!("socks5") >>
        space1 >>
        d: socket_addr >>
        ( EgressAddr::Socks5(d))
   )
);

named!(egress_interface<&[u8], EgressAddr>,
   do_parse!(
        tag_s!("bind") >>
        space1 >>
        d: ip_addr >>
        ( EgressAddr::From(d))
   )
);

named!(rule_conf<&[u8], Rule>,
    do_parse!(
        name: var_name >>
        equals >>
        v: dbg_dmp!(read_branch) >>
        ( Rule {
            name: name.into(),
          branch: v,
        } )
    )
);
named!(nameserver_value<&[u8], NameServer >,
    do_parse!(
        egress: opt!(do_parse!(
                         n: var_name >>
                         tag!("|") >>
                         ( n )
                     )) >>
        proto: map_res!( alt!(tag!("tcp")|tag!("udp")), str::from_utf8) >>
        space1 >>
        a: socket_addr >>
        ( NameServer {
            egress: egress.map(|e| RefVal::Ref(e.into())),
            remote: NameServerRemote::new(proto, a),
        } )
    )
);
named!(relay_conf<&[u8], Relay >,
    do_parse!(
        char!('{') >>
        opt_line_sep >>
        conf: permutation!(
            do_parse!(
                tag!("resolver") >>
                equals >>
                v: nameserver_value >>
                line_sep >>
                ( v )
            )?,
            do_parse!(
                tag!("listen") >>
                equals >>
                tag!("socks5") >>
                space1 >>
                a: socket_addr >>
                line_sep >>
                ( RelayProto::Socks5(a) )
            ),
            do_parse!(
                tag!("rule") >>
                equals >>
                n: var_name >>
                line_sep >>
                ( n )
            )
        ) >>
        char!('}') >>
        ( Relay {
             resolver: conf.0,
             listen: conf.1,
             rule: RefVal::Ref(conf.2.into()),
        } )
    )
);

named!(dns_conf<&[u8], DnsProxy >,
    do_parse!(
        char!('{') >> opt_line_sep >>
        conf: map_res!( permutation!(
            do_parse!(
                tag!("listen") >>
                equals >>
                tag!("udp") >>
                space1 >>
                v: socket_addr >>
                line_sep >>
                ( v   )
            ),
            do_parse!(
                tag!("forward") >> equals >> char!('{') >> opt_line_sep >>
                m: do_parse!(
                     entries: separated_nonempty_list!(line_sep, read_map_entry) >>
                    ( entries )
                ) >>
                opt_line_sep >> char!('}') >> line_sep >>
                ( m )
            )
        ), |(l, f)| DnsProxy::new1(l, f) ) >>
        char!('}') >>
        ( conf )
    )
);


named!(read_map_entry<&[u8], (Bytes, NameServer)>,
    do_parse!(
        keyword: var_name >>
        space0 >> tag!("=>") >> space0 >>
        value: nameserver_value >>
        ( (keyword.into(), value) )
    )
);

named!(ip_addr<&[u8], IpAddr>,
  map_res!(map_res!(
     take_while!( |c: u8| -> bool {
        c.is_ascii_hexdigit() || c == b'.' || c == b':'
     }), str::from_utf8),
  str::FromStr::from_str)
);

named!(socket_addr<&[u8], SocketAddr>,
  map_res!(map_res!(
     take_while!( |c: u8| -> bool {
        c.is_ascii_hexdigit() || c == b'.' || c == b':' || c == b'[' || c == b']'
     }), str::from_utf8),
  str::FromStr::from_str)
);


#[cfg(test)]
mod tests {
    use std::fs;
    use super::conf_items;
    use bytes::Bytes;

    #[test]
    fn test() {
        let f = fs::read("config/config").unwrap();
        match  conf_items(&f) {
            Ok((bytes, items)) => {
                println!("Sucessful parse: {:?}", items);
                let bs: Bytes = bytes.into();
                println!("Remaining: {:?}", bs);
            }
            Err(x) => eprintln!("parse failure {:?}", x),
        }
    }
}

impl fmt::Debug for Item {
    fn fmt(&self, f: & mut Formatter) -> Result<(), fmt::Error> {
        match self {
            Item::Egress(e) => write!(f, "Item {:?}", e),
            Item::Relay(x) => write!(f, "Item {:?}", x),
            Item::Dns(x) => write!(f, "Item {:?}", x),
            Item::Rule(x) => write!(f, "Item {:?}", x),
        }
    }
}