use std::net::SocketAddr;

use futures::Future;
use futures::future;

use super::client::socks::SockGetterAsync;
use super::client::udp::udp_get;
use futures_cpupool::CpuPool;
use std::io;
use super::config::DnsUpstream;

#[derive(Debug)]
pub enum  DnsClient {
    Direct(SocketAddr),
    ViaSocks5(SockGetterAsync),
}

impl DnsClient {
    pub fn new(up: &DnsUpstream, pool: &CpuPool)
        -> DnsClient{
        up.socks5.map(|a| {
            DnsClient::ViaSocks5(
                SockGetterAsync::new(pool.clone(), a, up.addr)
            )
        }).unwrap_or_else(|| {
            DnsClient::Direct(up.addr)
        })
    }

    pub fn resolve(&self, data: Vec<u8>)
        -> Box<Future<Item=Vec<u8>, Error=io::Error> + Send> {
        return match self {
            DnsClient::ViaSocks5(s) => {
                let f = s.get(data);
                f
            }
            DnsClient::Direct(s) => {
                let f = udp_get(s, data);
                flat_result_future(f)
            }
        }
    }
}

pub fn flat_result_future<E,F,FT,FE>(rf: Result<F,E>)->Box<Future<Item=FT, Error=FE>+Send>
    where F: Future<Item=FT,Error=FE> + Send + 'static,
          E: Into<FE>,
          FT: Send + 'static,
          FE: Send + 'static{
    match rf {
        Ok(f) => Box::new(f),
        Err(e) => {
            let e: FE = e.into();
            Box::new(future::err(e))
        }
    }
}