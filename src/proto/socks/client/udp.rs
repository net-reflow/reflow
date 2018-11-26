use tokio::net::{UdpSocket, TcpStream};
use std::net::{self, SocketAddr};
use crate::proto::socks::SocksError;
use crate::proto::socks::Address;
use std::net::SocketAddrV4;
use std::net::Ipv4Addr;
use crate::proto::socks::client::connect_socks_udp;
use std::io;
use bytes::{BytesMut, BufMut};
use crate::proto::socks::codec::write_address;
use crate::proto::socks::codec::read_address;
use byteorder::ReadBytesExt;
use std::time::Duration;
use tokio::reactor::Handle;
use std::io::Read;

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
    pub async fn bind_with_timeout(
        proxy: SocketAddr,
        local: SocketAddr,
        time: Option<Duration>) -> Result<Socks5Datagram, SocksError> {
        // we don't know what our IP is from the perspective of the proxy, so
        // don't try to pass `addr` in here.
        let dst = Address::SocketAddress(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0)));
        let mut stream = await!(TcpStream::connect(&proxy))?;
        let proxy_addr: Address = await!(connect_socks_udp(&mut stream, dst))?;
        let proxy_addr = match proxy_addr {
            Address::SocketAddress(a) => a,
            // I don't think a socks proxy will ever return a domain name in this case
            Address::DomainNameAddress(d, p) => {
                warn!("Received domain name address when connecting udp over socks: {}:{}", d, p);
                return Err(SocksError::ProtocolIncorrect)
            },
        };
        let socket = net::UdpSocket::bind(&local)?;
        if time.is_some() {
            socket.set_read_timeout(time)?;
        }

        let socket = UdpSocket::from_std(socket, &Handle::current())?;

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

    pub async fn recv_from(self, buf: &mut [u8])
        -> Result<(Address, usize), SocksError> {
        let mut header:Vec<u8> = vec![];
        header.resize(998, 0);
        let (_socket, header, len, addr) = await!(self.socket.recv_dgram(&mut header))?;
        trace!("received dgram with {} bytes from {:?}", len, addr);
        header.resize(len, 0);
        trace!("dgram {:?}", header);
        let mut cursor = io::Cursor::new(header);

        let mut rb = [0u8; 2];
        cursor.read_exact(&mut rb)?;
        if rb[0] != 0 || rb[1] != 0 {
            return Err(SocksError::InvalidData { msg: "invalid reserved bytes", data: rb.to_vec()});
        }
        let frag = cursor.read_u8()?;
        if frag != 0 {
            return Err(SocksError::InvalidData { msg: "invalid fragment id", data: vec![frag]});
        }
        let a = read_address(&mut cursor)?;
        trace!("read address {:?} with length {}", a, a.len());
        let n = cursor.read(buf)?;
        Ok((a, n))
    }

    pub fn get_ref(&self) -> &UdpSocket {
        &self.socket
    }
}
