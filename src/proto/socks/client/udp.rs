use tokio::net::{UdpSocket, TcpStream};
use std::net::SocketAddr;
use crate::proto::socks::SocksError;
use crate::proto::socks::Address;
use std::net::SocketAddrV4;
use std::net::Ipv4Addr;
use crate::proto::socks::client::connect_socks_udp;
use std::io;
use bytes::{BytesMut, BufMut};
use crate::proto::socks::codec::write_address;

#[derive(Debug)]
pub struct Socks5Datagram {
    socket: UdpSocket,
    proxy_addr: SocketAddr,
    // keeps the session alive
    stream: TcpStream,
}

impl Socks5Datagram {
    /// Creates a UDP socket bound to the specified address which will have its
    /// traffic routed through the specified proxy.
    pub async fn bind(proxy: SocketAddr, local: SocketAddr) -> Result<Socks5Datagram, SocksError> {
        // we don't know what our IP is from the perspective of the proxy, so
        // don't try to pass `addr` in here.
        let dst = Address::SocketAddress(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0)));
        let mut stream = await!(TcpStream::connect(&proxy))?;
        let proxy_addr: Address = await!(connect_socks_udp(&mut stream, dst))?;
        let proxy_addr = match proxy_addr {
            Address::SocketAddress(a) => a,
            // I don't think a socks proxy will ever return a domain name in this case
            Address::DomainNameAddress(_, _) => return Err(SocksError::ProtocolIncorrect),
        };

        let socket = UdpSocket::bind(&local)?;

        Ok(Socks5Datagram {
            socket: socket,
            proxy_addr,
            stream: stream,
        })
    }

    pub async fn send_to(self, d: &[u8], addr: Address) -> io::Result<Socks5Datagram> {
        let mut buf = BytesMut::with_capacity((&addr).len()+3+d.len());
        buf.put_slice(&[0, 0, // reserved
            0, // fragment id
        ]);
        write_address(&addr, &mut buf);
        buf.put_slice(d);
        let (s, _b) = await!(self.socket.send_dgram(buf, &self.proxy_addr))?;
        let new_self = Socks5Datagram {
            socket: s,
            proxy_addr: self.proxy_addr,
            stream: self.stream,
        };
        Ok(new_self)
    }
}
