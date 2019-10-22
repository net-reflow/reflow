use std::fmt;
use std::io;
use std::net::SocketAddr;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use tokio::net::TcpStream;

use crate::conf::NameServerRemote;
use asocks5::socks::Address;
use asocks5::socks::SocksError;
use asocks5::{connect_socks_socket_addr, Socks5Datagram};
use tokio::prelude::*;

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
        SockGetterAsync {
            proxy,
            addr: remote,
        }
    }

    pub async fn get(&self, message: Vec<u8>) -> Result<Vec<u8>, SocksError> {
        match self.addr {
            NameServerRemote::Tcp(_a) => self.get_tcp(message).await,
            NameServerRemote::Udp(_a) => self.get_udp(message).await,
        }
    }

    pub async fn get_udp(&self, data: Vec<u8>) -> Result<Vec<u8>, SocksError> {
        use std::str::FromStr;
        let la = SocketAddr::from_str("0.0.0.0:0").unwrap();
        let socks5 = Socks5Datagram::bind(self.proxy, la).await?;
        let addr = ns_sock_addr(&self.addr);
        let socks5 = socks5.send_to(&data, Address::SocketAddress(addr)).await?;

        let mut buf = vec![0; 998];
        let (addr, n) = socks5.recv_from(&mut buf).await?;
        debug!("receive {} from {:?}", n, addr);
        buf.truncate(n);
        Ok(buf)
    }

    pub async fn get_tcp(&self, data: Vec<u8>) -> Result<Vec<u8>, SocksError> {
        let target = ns_sock_addr(&self.addr);

        let mut stream = TcpStream::connect(&self.proxy).await?;
        connect_socks_socket_addr(&mut stream, target).await?;

        let len = data.len() as u16;
        let mut lens = [0u8; 2];
        lens.as_mut()
            .write_u16::<BigEndian>(len)
            .expect("byteorder");
        trace!("Sending length {}, {:?}", len, lens);
        stream.write_all(&lens).await?;
        stream.write_all(&data).await?;
        let mut b = [0u8; 2];
        stream.read_exact(&mut b).await?;
        trace!("Read reply length {:?}", b);
        let mut rdr = io::Cursor::new(b);
        let len = rdr.read_u16::<BigEndian>().expect("read u16");
        trace!("Reply length is {}", len);
        let mut buf = data;
        buf.resize(len as usize, 0);
        trace!("Resized buf size to {}", buf.len());
        if let Err(e) = stream.read_exact(&mut buf).await {
            warn!("Error reading {} bytes: {:?}", len, e);
            return Err(e.into());
        }
        Ok(buf)
    }
}

fn ns_sock_addr(ns: &NameServerRemote) -> SocketAddr {
    match ns {
        NameServerRemote::Udp(a) => *a,
        NameServerRemote::Tcp(a) => *a,
    }
}
