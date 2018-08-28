use std::net::SocketAddr;

use futures::Future;
use futures::future;

use super::client::socks::SockGetterAsync;
use super::client::udp::udp_get;
use futures_cpupool::CpuPool;
use std::io;
use conf::EgressAddr;
use std::net::IpAddr;
use std::net::UdpSocket as StdUdpSocket;
use tokio::net::UdpSocket;
use std::time::Duration;
use resolver::client::TIMEOUT;
use tokio::reactor::Handle;
use futures::future::Either;
use util::Either3;
use conf::NameServer;

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
                    DnsClient::DirectBind(up.remote.sock_addr(), i)
                }
            }
        } else {
            DnsClient::Direct(up.remote.sock_addr())
        }
    }

    pub fn resolve(&self, data: Vec<u8>)
        -> impl Future<Item=Vec<u8>, Error=io::Error> + Send {
        return match self {
            DnsClient::ViaSocks5(s) => {
                let f = s.get(data);
                Either3::A(f)
            }
            // FIXME these always use udp even when tcp is configured
            DnsClient::Direct(s) => {
                let f = udp_get(s, data);
                Either3::B(flat_result_future(f))
            }
            DnsClient::DirectBind(s, i) => Either3::C(udp_bind_get(*s, *i, data))
        }
    }
}

pub fn flat_result_future<E,F,FT,FE>(rf: Result<F,E>)->impl Future<Item=FT, Error=FE>+Send
    where F: Future<Item=FT,Error=FE> + Send + 'static,
          E: Into<FE>,
          FT: Send + 'static,
          FE: Send + 'static{
    match rf {
        Ok(f) => Either::A(f),
        Err(e) => {
            let e: FE = e.into();
            Either::B(future::err(e))
        }
    }
}

pub fn udp_bind_get(addr: SocketAddr, ip: IpAddr, data: Vec<u8>)
               -> impl Future<Item=Vec<u8>,Error=io::Error> + Send {
    use futures::IntoFuture;
    StdUdpSocket::bind(SocketAddr::from((ip, 0)))
        .into_future()
        .and_then(|s| {
            let _ = s.set_read_timeout(Some(Duration::from_secs(TIMEOUT)));
            Ok(s)
        })
        .and_then(|s| UdpSocket::from_std(s, &Handle::current()))
        .and_then(move|s| s.send_dgram(data, &addr))
        .and_then(|(s, _b)| {
            let buf = vec![0; 998];
            s.recv_dgram(buf)
        })
        .and_then(|(_s, b, nb, _a)| Ok(b[..nb].into()))
}