use std::io;
use std::fmt;
use std::net::SocketAddr;

use byteorder::{BigEndian,WriteBytesExt,ReadBytesExt};
use socks::Socks5Datagram;
use std::time::Duration;
use futures_cpupool::CpuPool;
use tokio::net::TcpStream;
use tokio_io::io as tio;

use super::TIMEOUT;

use crate::conf::NameServerRemote;
use crate::proto::socks::connect_socks_to;
use crate::proto::socks::Address;
use crate::proto::socks::SocksError;

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

    pub async fn get(&self, message: Vec<u8>) -> Result<Vec<u8>, SocksError> {
        match self.addr {
            NameServerRemote::Tcp(_a) => {
                await!(self.get_tcp(message))
            }
            NameServerRemote::Udp(_a) => {
                await!(self.get_udp(message))
            }
        }
    }

    pub async fn get_udp(&self, data: Vec<u8>)
                   -> Result<Vec<u8>, SocksError> {
        let socks5 = Socks5Datagram::bind(self.proxy, "0.0.0.0:0")?;
        socks5.get_ref().set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;
        let addr = ns_sock_addr(&self.addr);
        let socks5 = await!(self.pool.spawn_fn(move || -> io::Result<Socks5Datagram> {
            let _n_b = socks5.send_to(&data, addr)?;
            Ok(socks5)
        }))?;
        let recv = await!(self.pool.spawn_fn(move || -> io::Result<Vec<u8>> {
            let mut buf = vec![0; 998];
            let (n, _addr) = socks5.recv_from(&mut buf)?;
            buf.truncate(n);
            Ok(buf)
        }))?;
        Ok(recv)
    }

    pub async fn get_tcp(&self, data: Vec<u8>)
                   -> Result<Vec<u8>, SocksError> {
        let target = ns_sock_addr(&self.addr);

        let mut stream = await!(TcpStream::connect(&self.proxy))?;
        await!(connect_socks_to(&mut stream, Address::SocketAddress(target)))?;

        let len = data.len() as u16;
        let mut lens = [0u8; 2];
        lens.as_mut().write_u16::<BigEndian>(len).expect("byteorder");
        trace!("Sending length {}, {:?}", len, lens);
        let (_ts, _b) = await!(tio::write_all(&mut stream, lens))?;
        let (_ts, mut buf) = await!(tio::write_all(&mut stream, data))?;
        let lb = [0u8; 2];
        let (_ts, b) = await!(tio::read_exact(&mut stream, lb))?;
        trace!("Read reply length {:?}", b);
        let mut rdr = io::Cursor::new(b);
        let len = rdr.read_u16::<BigEndian>().expect("read u16");
        trace!("Reply length is {}", len);
        buf.resize(len as usize, 0);
        trace!("Resized buf size to {}", buf.len());
        let (_s, buf) = await!(tio::read_exact(&mut stream, buf)).map_err(|e| {
            warn!("Error reading {} bytes: {:?}", len, e);
            e
        })?;
        Ok(buf)
    }
}

fn ns_sock_addr(ns: &NameServerRemote) -> SocketAddr {
    match ns {
        NameServerRemote::Udp(a) => *a,
        NameServerRemote::Tcp(a) => *a,
    }
}
