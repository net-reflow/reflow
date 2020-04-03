use std::net::SocketAddr;

use super::client::socks::SockGetterAsync;
use super::client::udp::udp_get;
use crate::conf::EgressAddr;
use crate::conf::NameServer;
use crate::conf::NameServerRemote;
use crate::resolver::client::TIMEOUT;
use asocks5::socks::SocksError;

use std::io;
use std::net::IpAddr;
use std::net::UdpSocket as StdUdpSocket;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio_net::driver::Handle;

#[derive(Debug)]
pub enum DnsClient {
    Direct(SocketAddr),
    DirectBind(SocketAddr, IpAddr),
    ViaSocks5(SockGetterAsync),
}

impl DnsClient {
    pub fn new(up: &NameServer) -> DnsClient {
        if let Some(ref e) = up.egress {
            let e = e.val();
            match e.addr() {
                EgressAddr::Socks5(s) => {
                    DnsClient::ViaSocks5(SockGetterAsync::new(s, up.remote.clone()))
                }
                EgressAddr::From(i) => {
                    let a = ns_sock_addr(&up.remote);
                    DnsClient::DirectBind(a, i)
                }
            }
        } else {
            DnsClient::Direct(ns_sock_addr(&up.remote))
        }
    }

    pub async fn resolve(&self, data: Vec<u8>) -> Result<Vec<u8>, SocksError> {
        match self {
            DnsClient::ViaSocks5(s) => s.get(data).await,
            // FIXME these always use udp even when tcp is configured
            DnsClient::Direct(s) => {
                let vec = udp_get(s, data).await?;
                Ok(vec)
            }
            DnsClient::DirectBind(s, i) => {
                let vec = udp_bind_get(*s, *i, data).await?;
                Ok(vec)
            }
        }
    }
}

fn ns_sock_addr(ns: &NameServerRemote) -> SocketAddr {
    match ns {
        NameServerRemote::Udp(a) => *a,
        NameServerRemote::Tcp(a) => {
            warn!("Dns over TCP isn't support yet even though it's configured");
            *a
        }
    }
}

pub async fn udp_bind_get(addr: SocketAddr, ip: IpAddr, data: Vec<u8>) -> io::Result<Vec<u8>> {
    let s = StdUdpSocket::bind(SocketAddr::from((ip, 0)))?;
    s.set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;
    let mut s = UdpSocket::from_std(s)?;
    s.send_to(&data, &addr).await?;
    let mut buf = vec![0; 998];
    let (nb, _a) = s.recv_from(&mut buf).await?;
    Ok(buf[..nb].into())
}
