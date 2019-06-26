//! Socks5 protocol definition (RFC1928)
//!
//! Implements [SOCKS Protocol Version 5](https://www.ietf.org/rfc/rfc1928.txt) proxy protocol

use std::convert;
use std::convert::TryInto;
use std::fmt::{self, Debug, Formatter};
use std::io;
use std::net::Shutdown;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::u8;

use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use failure::Fail;
use futures::compat::Future01CompatExt;

use tokio_io::io::read_exact;

use crate::consts;
use crate::consts::Reply;
use crate::Command;
use tokio::net::TcpStream;

#[derive(Debug, Fail)]
pub enum SocksError {
    #[fail(display = "Not supported socks version {}", ver)]
    SocksVersionNoSupport { ver: u8 },
    #[fail(display = "Not supported address type {}", code)]
    AddressTypeNotSupported { code: u8 },
    #[fail(display = "Invalid domain name encoding in address")]
    InvalidDomainEncoding,
    #[fail(display = "No supported auth methods")]
    NoSupportAuth,
    #[fail(display = "Unsupported command {}", cmd)]
    CommandUnSupport { cmd: u8 },
    #[fail(display = "Invalid reply {}", reply)]
    InvalidReply { reply: u8 },
    #[fail(display = "Server replied error {:?}", reply)]
    RepliedError { reply: Reply },
    #[fail(display = "Violation of the socks protocol")]
    ProtocolIncorrect,
    #[fail(display = "Invalid data {}: {:?}", msg, data)]
    InvalidData { msg: &'static str, data: Vec<u8> },
    #[fail(display = "IO Error {:?}", err)]
    IOError { err: io::Error },
}

impl convert::From<io::Error> for SocksError {
    fn from(err: io::Error) -> Self {
        SocksError::IOError { err }
    }
}

/// SOCKS5 address type
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Address {
    /// Socket address (IP Address)
    SocketAddress(SocketAddr),
    /// Domain name address
    DomainNameAddress(String, u16),
}

impl Address {
    pub fn len(&self) -> usize {
        match *self {
            Address::SocketAddress(SocketAddr::V4(..)) => 1 + 4 + 2,
            Address::SocketAddress(SocketAddr::V6(..)) => 1 + 8 * 2 + 2,
            Address::DomainNameAddress(ref dmname, _) => 1 + 1 + dmname.len() + 2,
        }
    }
}

impl Debug for Address {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Address::SocketAddress(ref addr) => write!(f, "{}", addr),
            Address::DomainNameAddress(ref addr, ref port) => write!(f, "{}:{}", addr, port),
        }
    }
}

pub async fn read_socks_address(mut stream: &mut TcpStream) -> Result<Address, SocksError> {
    let a = await!(read_exact(&mut stream, [0u8; 1]).compat())?.1[0];
    let addr_type: consts::AddrType = a.try_into()?;
    match addr_type {
        consts::AddrType::IPV4 => {
            let buf = [0u8; 4 + 2];
            let (_s, buf) = await!(read_exact(&mut stream, buf).compat())?;
            let mut cursor = io::Cursor::new(buf);
            let v4addr = Ipv4Addr::from(cursor.read_u32::<BigEndian>()?);
            let port = cursor.read_u16::<BigEndian>()?;
            let addr = Address::SocketAddress(SocketAddr::V4(SocketAddrV4::new(v4addr, port)));
            Ok(addr)
        }
        consts::AddrType::IPV6 => {
            let buf = [0u8; 16];
            let (_s, buf) = await!(read_exact(&mut stream, buf).compat())?;
            let v6addr = Ipv6Addr::from(buf);
            let buf = [0u8; 2];
            let (_s, buf) = await!(read_exact(&mut stream, buf).compat())?;
            let mut cursor = io::Cursor::new(buf);
            let port = cursor.read_u16::<BigEndian>()?;

            let addr =
                Address::SocketAddress(SocketAddr::V6(SocketAddrV6::new(v6addr, port, 0, 0)));
            Ok(addr)
        }
        consts::AddrType::DomainName => {
            // domain and port
            let length = await!(read_exact(&mut stream, [0u8; 1]).compat())?.1[0];
            let addr_len = length as usize;
            let mut raw_addr = [0u8; 257];
            await!(read_exact(&mut stream, &mut raw_addr[..addr_len + 2]).compat())?;
            let mut cursor = io::Cursor::new(&raw_addr[addr_len..]);
            let port = cursor.read_u16::<BigEndian>()?;
            let addr = String::from_utf8_lossy(&raw_addr[..addr_len]);
            let addr = Address::DomainNameAddress(addr.into(), port);
            Ok(addr)
        }
    }
}

pub async fn read_socks_socket_addr(mut stream: &mut TcpStream) -> Result<SocketAddr, SocksError> {
    let a = await!(read_exact(&mut stream, [0u8; 1]).compat())?.1[0];
    let addr_type: consts::AddrType = a.try_into()?;
    match addr_type {
        consts::AddrType::IPV4 => {
            let buf = [0u8; 4 + 2];
            let (_s, buf) = await!(read_exact(&mut stream, buf).compat())?;
            let mut cursor = io::Cursor::new(buf);
            let v4addr = Ipv4Addr::from(cursor.read_u32::<BigEndian>()?);
            let port = cursor.read_u16::<BigEndian>()?;
            let addr = SocketAddr::V4(SocketAddrV4::new(v4addr, port));
            Ok(addr)
        }
        consts::AddrType::IPV6 => {
            let buf = [0u8; 16];
            let (_s, buf) = await!(read_exact(&mut stream, buf).compat())?;
            let v6addr = Ipv6Addr::from(buf);
            let buf = [0u8; 2];
            let (_s, buf) = await!(read_exact(&mut stream, buf).compat())?;
            let mut cursor = io::Cursor::new(buf);
            let port = cursor.read_u16::<BigEndian>()?;

            let addr = SocketAddr::V6(SocketAddrV6::new(v6addr, port, 0, 0));
            Ok(addr)
        }
        t => Err(SocksError::AddressTypeNotSupported { code: t as u8 }),
    }
}

/// TCP request header after handshake
///
/// ```plain
/// +----+-----+-------+------+----------+----------+
/// |VER | CMD |  RSV  | ATYP | DST.ADDR | DST.PORT |
/// +----+-----+-------+------+----------+----------+
/// | 1  |  1  | X'00' |  1   | Variable |    2     |
/// +----+-----+-------+------+----------+----------+
/// ```
#[derive(Clone, Debug)]
pub struct TcpRequestHeader {
    /// SOCKS5 command
    pub command: Command,
    /// Remote address
    pub address: Address,
}

/// TCP response header
///
/// ```plain
/// +----+-----+-------+------+----------+----------+
/// |VER | REP |  RSV  | ATYP | BND.ADDR | BND.PORT |
/// +----+-----+-------+------+----------+----------+
/// | 1  |  1  | X'00' |  1   | Variable |    2     |
/// +----+-----+-------+------+----------+----------+
/// ```
#[derive(Clone, Debug)]
pub struct TcpResponseHeader {
    /// SOCKS5 reply
    pub reply: Reply,
    /// Reply address
    pub address: Address,
}

impl TcpResponseHeader {
    /// Creates a response header
    pub fn new(reply: Reply, address: Address) -> TcpResponseHeader {
        TcpResponseHeader { reply, address }
    }

    /// Length in bytes
    #[inline]
    pub fn len(&self) -> usize {
        self.address.len() + 3
    }
}

/// SOCKS5 handshake request packet
///
/// ```plain
/// +----+----------+----------+
/// |VER | NMETHODS | METHODS  |
/// +----+----------+----------+
/// |'05'|    1     | 1 to 255 |
/// +----+----------+----------|
/// ```
#[derive(Clone, Debug)]
pub struct HandshakeRequest {
    pub methods: Vec<u8>,
}

/// Read from a reader
pub async fn read_handshake_request(mut s: &mut TcpStream) -> Result<HandshakeRequest, SocksError> {
    let (_s, buf) = await!(read_exact(&mut s, [0u8, 0u8]).compat())?;
    let ver = buf[0];
    let nmet = buf[1];

    if ver != consts::SOCKS5_VERSION {
        s.shutdown(Shutdown::Both)?;
        return Err(SocksError::SocksVersionNoSupport { ver });
    }
    let (_s, methods) = await!(read_exact(&mut s, vec![0u8; nmet as usize]).compat())?;
    Ok(HandshakeRequest { methods })
}

/// SOCKS5 handshake response packet
///
/// ```plain
/// +----+--------+
/// |VER | METHOD |
/// +----+--------+
/// | 1  |   1    |
/// +----+--------+
/// ```
#[derive(Clone, Debug, Copy)]
pub struct HandshakeResponse {
    pub chosen_method: u8,
}
