use std::net::IpAddr;
use std::net::SocketAddr;
use std::fmt;
use crate::conf::RoutingBranch;
use crate::conf::main::util::RefVal;
use crate::conf::main::dns::NameServer;
use crate::conf::NameServerRemote;

pub struct Relay {
    pub resolver: Option<NameServer>,
    pub listen: RelayProto,
    pub rule: RefVal<RoutingBranch>,
}
impl fmt::Display for Relay {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Relay on {:?}", self.listen)?;
        Ok(())
    }
}

impl fmt::Debug for Relay {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Relay on {:?},", self.listen)?;
        write!(f, "resolver: {:?},", self.resolver)?;
        write!(f, "rule: {:?},", self.rule)?;
        Ok(())
    }
}

pub enum RelayProto {
    Socks5(SocketAddr),
}


impl Relay {
    pub fn nameserver_or_default(&self)-> NameServer {
        match self.resolver {
            Some(ref r) => r.clone(),
            None => {
                NameServer {
                    egress: None,
                    remote: NameServerRemote::Udp(
                        SocketAddr::new(
                            IpAddr::from([8,8,8,8]),
                            53)),
                }
            }
        }
    }
}

impl fmt::Debug for RelayProto {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            RelayProto::Socks5(a) => write!(f, "socks5 {:?}", a)?,
        }
        Ok(())
    }
}