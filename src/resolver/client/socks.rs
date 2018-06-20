use std::io;
use std::net::SocketAddr;

use futures::{Future};
use socks::Socks5Datagram;
use std::time::Duration;
use futures_cpupool::CpuPool;

use super::TIMEOUT;

pub struct SockGetterAsync {
    pool: CpuPool,
    proxy: SocketAddr,
}

impl SockGetterAsync {
    pub fn new(pool: CpuPool, proxy: SocketAddr)-> SockGetterAsync {
        let sga = SockGetterAsync {
            pool: pool,
            proxy: proxy,
        };
        sga
    }

    pub fn get(&self, addr: SocketAddr, data: Vec<u8>)
                          -> io::Result<impl Future<Item=Vec<u8>, Error=io::Error> + Send> {
        let socks5 = Socks5Datagram::bind(self.proxy, "0.0.0.0:0")?;
        socks5.get_ref().set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;
        let sendf = self.pool.spawn_fn(move || -> io::Result<Socks5Datagram> {
            let _n_b = socks5.send_to(&data, addr)?;
            Ok(socks5)
        });
        let pool = self.pool.clone();
        let recvf =
            sendf.and_then(move |s| {
                pool.spawn_fn(move || -> io::Result<Vec<u8>> {
                    let mut buf = vec![0; 998];
                    let (n, _addr) = s.recv_from(&mut buf)?;
                    buf.truncate(n);
                    Ok(buf)
                })
            });
        Ok(recvf)
    }
}