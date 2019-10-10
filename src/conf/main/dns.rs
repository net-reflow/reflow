use crate::conf::main::util::RefVal;
use crate::conf::Egress;
use crate::util::BsDisp;
use bytes::Bytes;
use failure::Error;
use std::collections::BTreeMap;
use std::fmt;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
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
pub enum NameServerRemote {
    Udp(SocketAddr),
    Tcp(SocketAddr),
}

impl DnsProxy {
    pub fn new1(listen: SocketAddr, ms: Vec<(Bytes, NameServer)>) -> Result<DnsProxy, Error> {
        let mut forward: BTreeMap<Bytes, NameServer> = ms.into_iter().collect();
        let b: Bytes = "else".into();
        let d = forward
            .remove(&b)
            .ok_or_else(|| format_err!("No default dns"))?;
        Ok(DnsProxy {
            listen,
            forward,
            default: d,
        })
    }

    /// replace named gateways with actual values
    pub fn deref_route(&mut self, gw: &BTreeMap<Bytes, Egress>) -> Result<(), Error> {
        for ns in &mut self.forward.values_mut() {
            if let Some(ref mut e) = ns.egress {
                e.insert_value(gw).map_err(|e| {
                    format_err!(
                        "Error in dns proxy configuration: \
                         {} is not a defined egress",
                        BsDisp::new(&e)
                    )
                })?;
            }
        }
        if let Some(ref mut e) = self.default.egress {
            e.insert_value(gw).map_err(|e| {
                format_err!(
                    "Error in dns proxy configuration: \
                     {} is not a defined egress",
                    BsDisp::new(&e)
                )
            })?;
        }
        Ok(())
    }
}
impl NameServerRemote {
    pub fn new(proto: &str, addr: SocketAddr) -> NameServerRemote {
        match proto {
            "tcp" => NameServerRemote::Tcp(addr),
            "udp" => NameServerRemote::Udp(addr),
            _ => panic!("{} is not known DNS protocol", proto),
        }
    }
}

impl fmt::Display for DnsProxy {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Dns proxy listening on on {:?}", self.listen)?;
        Ok(())
    }
}
