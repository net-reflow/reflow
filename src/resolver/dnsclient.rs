use std::net::SocketAddr;
use std::fmt;

use futures::Future;
use futures::future;
use trust_dns::rr::LowerName;

use super::client::socks::SockGetterAsync;
use super::client::udp::udp_get;
use futures_cpupool::CpuPool;
use std::io;

pub struct DnsClient {
    server: SocketAddr,
    proxy: Option<SockGetterAsync>,
}

impl fmt::Display for DnsClient {
    fn fmt(&self, fmt: &mut fmt::Formatter)-> fmt::Result {
        write!(fmt, "{}", self.server)
    }
}

impl DnsClient {
    pub fn new(sa: SocketAddr, proxy_addr: Option<SocketAddr>, pool: &CpuPool)
        -> DnsClient{
        let proxy = proxy_addr.map(|a| {
            SockGetterAsync::new(pool.clone(), a)
        });
        let c = DnsClient {
            server: sa,
            proxy,
        };
        c
    }

    pub fn resolve(&self, data: Vec<u8>, name: &LowerName)
        -> Box<Future<Item=Vec<u8>, Error=io::Error> + Send> {
        if let Some(ref s) = self.proxy {
            let f = s.get(self.server, data);
            return flat_result_future(f);
        } else {
            let f = udp_get(&self.server, data);
            return flat_result_future(f);
        }
    }
}

fn flat_result_future<E,F,FT,FE>(rf: Result<F,E>)->Box<Future<Item=FT, Error=FE>+Send>
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