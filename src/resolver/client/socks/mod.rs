use std::io;
use std::fmt;
use std::net::SocketAddr;

use byteorder::{BigEndian,WriteBytesExt,ReadBytesExt};
use futures::{Future};
use socks::Socks5Datagram;
use socks::Socks5Stream;
use std::time::Duration;
use futures_cpupool::CpuPool;
use tokio::net::TcpStream;
use tokio::reactor::Handle;
use tokio_io::io as tio;

use super::TIMEOUT;

use super::super::dnsclient::flat_result_future;
use futures::future::Either;
use conf::NameServerRemote;

/// Do dns queries through a socks5 proxy
pub struct SockGetterAsync {
    pool: CpuPool,
    proxy: SocketAddr,
    addr: NameServerRemote,
}

impl fmt::Debug for SockGetterAsync {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Socks({:?})|Dns({:?})", self.proxy, self.addr)
    }
}


impl SockGetterAsync {
    pub fn new(pool: CpuPool, proxy: SocketAddr, remote: NameServerRemote)-> SockGetterAsync {
        let sga = SockGetterAsync {
            pool: pool,
            proxy: proxy,
            addr: remote,
        };
        sga
    }

    pub fn get(&self, message: Vec<u8>) -> impl Future<Item=Vec<u8>, Error=io::Error> + Send {
        match self.addr {
            NameServerRemote::Tcp(_a) => {
                Either::A(flat_result_future(self.get_tcp(message)))
            }
            NameServerRemote::Udp(_a) => {
                Either::B(flat_result_future(self.get_udp(message)))
            }
        }
    }

    pub fn get_udp(&self, data: Vec<u8>)
                   -> io::Result<impl Future<Item=Vec<u8>, Error=io::Error> + Send> {
        let socks5 = Socks5Datagram::bind(self.proxy, "0.0.0.0:0")?;
        socks5.get_ref().set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;
        let addr = self.addr.sock_addr();
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

    pub fn get_tcp(&self, data: Vec<u8>)
                   -> io::Result<impl Future<Item=Vec<u8>, Error=io::Error> + Send> {
        let proxy = self.proxy.clone();
        let target = self.addr.sock_addr();
        let connectf = self.pool.spawn_fn(move || -> io::Result<Socks5Stream> {
            let ss = Socks5Stream::connect(proxy, target)?;
            ss.get_ref().set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;
            Ok(ss)
        });
        let streamf = connectf.and_then(|ss| {
            let ts = ss.into_inner();
            TcpStream::from_std(ts, &Handle::current())
        });
        let len = data.len() as u16;
        let mut lens = [0u8; 2];
        lens.as_mut().write_u16::<BigEndian>(len).expect("byteorder");
        debug!("Sending length {}, {:?}", len, lens);
        let rep =
            streamf.and_then(move |ts| tio::write_all(ts, lens))
            .and_then(move |(ts, _b)| {
                tio::write_all(ts, data)
            })
            .and_then(|(ts, buf)| {
                let lb = [0u8; 2];
                tio::read_exact(ts,lb).map(move |(s, b)| {
                    debug!("Read reply length {:?}", b);
                    let mut rdr = io::Cursor::new(b);
                    let len = rdr.read_u16::<BigEndian>().expect("read u16");
                    debug!("Reply length is {}", len);
                    (s, len, buf)
                })
            })
            .and_then(|(ts, len, mut buf)| {
                buf.resize(len as usize, 0);
                debug!("Resized buf size to {}", buf.len());
                tio::read_exact(ts, buf)
                    .map(|(_s, buf)| buf)
                    .map_err(move |e| {
                        warn!("Error reading {} bytes: {:?}", len, e);
                        e
                    })
            })

        ;
        Ok(rep)
    }
}