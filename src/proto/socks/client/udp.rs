use tokio::net::{UdpSocket, TcpStream};
use std::net::SocketAddr;
use crate::proto::socks::SocksError;
use crate::proto::socks::Address;
use std::net::SocketAddrV4;
use std::net::Ipv4Addr;
use crate::proto::socks::client::connect_socks_udp;

#[derive(Debug)]
pub struct Socks5Datagram {
    socket: UdpSocket,
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
        socket.connect(&proxy_addr)?;

        Ok(Socks5Datagram {
            socket: socket,
            stream: stream,
        })
    }
}