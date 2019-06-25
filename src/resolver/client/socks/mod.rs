use std::fmt;
use std::io;
use std::net::SocketAddr;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use futures::compat::Future01CompatExt;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_io::io as tio;

use super::TIMEOUT;

use crate::conf::NameServerRemote;
use asocks5::socks::Address;
use asocks5::socks::SocksError;
use asocks5::{Socks5Datagram, connect_socks_socket_addr};

use tokio::prelude::future::Future;

/// Do dns queries through a socks5 proxy
pub struct SockGetterAsync {
    proxy: SocketAddr,
    addr: NameServerRemote,
}

impl fmt::Debug for SockGetterAsync {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Socks({:?})|Dns({:?})", self.proxy, self.addr)
    }
}

impl SockGetterAsync {
    pub fn new(proxy: SocketAddr, remote: NameServerRemote) -> SockGetterAsync {
        let sga = SockGetterAsync {
            proxy: proxy,
            addr: remote,
        };
        sga
    }

    pub async fn get(&self, message: Vec<u8>) -> Result<Vec<u8>, SocksError> {
        match self.addr {
            NameServerRemote::Tcp(_a) => await!(self.get_tcp(message)),
            NameServerRemote::Udp(_a) => await!(self.get_udp(message)),
        }
    }

    pub async fn get_udp(&self, data: Vec<u8>) -> Result<Vec<u8>, SocksError> {
        use std::str::FromStr;
        let la = SocketAddr::from_str("0.0.0.0:0").unwrap();
        let socks5 = await!(Socks5Datagram::bind_with_timeout(
            self.proxy,
            la,
            Some(Duration::from_secs(TIMEOUT))
        ))?;
        let addr = ns_sock_addr(&self.addr);
        let socks5 = await!(socks5.send_to(&data, Address::SocketAddress(addr)))?;

        let mut buf = vec![0; 998];
        let (addr, n) = await!(socks5.recv_from(&mut buf))?;
        debug!("receive {} from {:?}", n, addr);
        buf.truncate(n);
        Ok(buf)
    }

    pub async fn get_tcp(&self, data: Vec<u8>) -> Result<Vec<u8>, SocksError> {
        let target = ns_sock_addr(&self.addr);

        let mut stream = await!(TcpStream::connect(&self.proxy).compat())?;
        connect_socks_socket_addr(&mut stream, target).await?;

        let len = data.len() as u16;
        let mut lens = [0u8; 2];
        lens.as_mut()
            .write_u16::<BigEndian>(len)
            .expect("byteorder");
        trace!("Sending length {}, {:?}", len, lens);
        let (_ts, _b) = await!(tio::write_all(&mut stream, lens).compat())?;
        let (_ts, mut buf) = await!(tio::write_all(&mut stream, data).compat())?;
        let lb = [0u8; 2];
        let (_ts, b) = await!(tio::read_exact(&mut stream, lb).compat())?;
        trace!("Read reply length {:?}", b);
        let mut rdr = io::Cursor::new(b);
        let len = rdr.read_u16::<BigEndian>().expect("read u16");
        trace!("Reply length is {}", len);
        buf.resize(len as usize, 0);
        trace!("Resized buf size to {}", buf.len());
        let (_s, buf) = await!(tio::read_exact(&mut stream, buf)
            .map_err(|e| {
                warn!("Error reading {} bytes: {:?}", len, e);
                e
            })
            .compat())?;
        Ok(buf)
    }
}

fn ns_sock_addr(ns: &NameServerRemote) -> SocketAddr {
    match ns {
        NameServerRemote::Udp(a) => *a,
        NameServerRemote::Tcp(a) => *a,
    }
}
