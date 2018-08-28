use std::net::SocketAddr;
use conf::Egress;
use conf::main::util::RefVal;
use bytes::Bytes;
use failure::Error;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct DnsProxy {
    pub listen: SocketAddr,
    pub forward: BTreeMap<Bytes, NameServer>,
    pub default: NameServer,
}

#[derive(Clone, Debug)]
pub struct NameServer {
    pub egress: Option<RefVal<Egress>>,
    pub remote: NameServerRemote,
}
#[derive(Clone, Debug)]
pub enum NameServerRemote{
    Udp(SocketAddr),
    Tcp(SocketAddr),
}
impl NameServerRemote {
    pub fn sock_addr(&self) -> SocketAddr {
        match self {
            NameServerRemote::Udp(a) => *a,
            NameServerRemote::Tcp(a) => *a,
        }
    }
}


impl DnsProxy {
    pub fn new1(listen: SocketAddr, ms: Vec<(Bytes,NameServer)>)->Result<DnsProxy, Error> {
        let mut forward: BTreeMap<Bytes, NameServer> = ms.into_iter().collect();
        let b: Bytes = "else".into();
        let d = forward.remove(&b)
            .ok_or_else(||format_err!("No default dns"))?;
        Ok(DnsProxy {
            listen: listen,
            forward: forward,
            default:d,
        })
    }

    /// replace named gateways with actual values
    fn deref_route(mut self, gw: &BTreeMap<Bytes, Egress>)
                   -> Result<DnsProxy, Error> {
        let f = self.forward.into_iter().map(|(k, mut v)| {
            if let Some(ref mut e) = v.egress {
                e.insert_value(gw).unwrap();
            }
            (k, v)
        }).collect();
        if let Some(ref mut e) = self.default.egress {
            e.insert_value(gw)?;
        }
        Ok(DnsProxy {
            listen: self.listen,
            forward: f,
            default: self.default,
        })
    }
}
impl NameServerRemote {
    pub fn new(proto: &str, addr: SocketAddr)->NameServerRemote {
        match proto {
            "tcp" => NameServerRemote::Tcp(addr),
            "udp" => NameServerRemote::Udp(addr),
            _ => panic!("{} is not known DNS protocol", proto),
        }
    }
}