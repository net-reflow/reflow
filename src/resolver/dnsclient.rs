use std::net::SocketAddr;

use super::client::socks::SockGetterAsync;
use super::client::udp::udp_get;
use futures_cpupool::CpuPool;
use std::io;
use crate::conf::EgressAddr;
use std::net::IpAddr;
use std::net::UdpSocket as StdUdpSocket;
use tokio::net::UdpSocket;
use std::time::Duration;
use crate::resolver::client::TIMEOUT;
use tokio::reactor::Handle;
use crate::conf::NameServer;
use crate::conf::NameServerRemote;

#[derive(Debug)]
pub enum DnsClient {
    Direct(SocketAddr),
    DirectBind(SocketAddr, IpAddr),
    ViaSocks5(SockGetterAsync),
}

impl DnsClient {
    pub fn new(up: &NameServer, pool: &CpuPool)
               -> DnsClient{
        if let Some(ref e) = up.egress {
            let e = e.val();
            match e.addr() {
                EgressAddr::Socks5(s) => {
                    DnsClient::ViaSocks5(
                        SockGetterAsync::new(pool.clone(), s, up.remote.clone())
                    )
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

    pub async fn resolve(&self, data: Vec<u8>)
        -> io::Result<Vec<u8>> {
        return match self {
            DnsClient::ViaSocks5(s) => {
                await!(s.get(data))
            }
            // FIXME these always use udp even when tcp is configured
            DnsClient::Direct(s) => await!(udp_get(s, data)),
            DnsClient::DirectBind(s, i) => await!(udp_bind_get(*s, *i, data))
        };
    }
}


fn ns_sock_addr(ns: &NameServerRemote) -> SocketAddr {
    match ns {
        NameServerRemote::Udp(a) => a.clone(),
        NameServerRemote::Tcp(a) => {
            warn!("Dns over TCP isn't support yet even though it's configured");
            a.clone()
        },
    }
}

pub async fn udp_bind_get(addr: SocketAddr, ip: IpAddr, data: Vec<u8>)
               -> io::Result<Vec<u8>> {
    let s = StdUdpSocket::bind(SocketAddr::from((ip, 0)))?;
    s.set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;
    let s = UdpSocket::from_std(s, &Handle::current())?;
    let (s, _b) = await!(s.send_dgram(data, &addr))?;
    let buf = vec![0; 998];
    let (_s, b, nb, _a) = await!(s.recv_dgram(buf))?;
    Ok(b[..nb].into())
}