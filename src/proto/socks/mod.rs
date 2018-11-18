//! Socks5 protocol definition (RFC1928)
//!
//! Implements [SOCKS Protocol Version 5](https://www.ietf.org/rfc/rfc1928.txt) proxy protocol

use std::convert::TryInto;
use std::convert;
use std::fmt::{self, Debug, Formatter};
use std::io::{self};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::net::Shutdown;
use std::u8;

use byteorder::{BigEndian};

use failure::Error;
use tokio_io::io::read_exact;

mod consts;
pub mod listen;
mod heads;
mod codec;
mod client;

pub use self::consts::Command;

use self::consts::Reply;
use tokio::net::TcpStream;
use byteorder::ByteOrder;

#[derive(Debug, Fail)]
pub enum SocksError {
    #[fail(display = "Not supported socks version {}", ver)]
    SocksVersionNoSupport { ver: u8 },
    #[fail(display = "Not supported address type {}", code)]
    AddressTypeNotSupported { code: u8},
    #[fail(display="Invalid domain name encoding in address")]
    InvalidDomainEncoding,
    #[fail(display="No supported auth methods")]
    NoSupportAuth,
    #[fail(display="Unsupported command {}", cmd)]
    CommandUnSupport { cmd: u8 },
    #[fail(display="Invalid reply {}", reply)]
    InvalidReply { reply: u8 },
    #[fail(display="Server replied error {:?}", reply)]
    RepliedError { reply: Reply },
    #[fail(display="Violation of the socks protocol")]
    ProtocolIncorrect,
    #[fail(display="IO Error {:?}", err)]
    IOError { err: io::Error },
}

impl convert::From<io::Error> for SocksError {
    fn from(err: io::Error) -> Self {
        SocksError::IOError {err}
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

    fn len(&self) -> usize {
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

async fn read_u8(stream: &mut TcpStream) -> io::Result<u8> {
    let (_s, b) = await!(read_exact(stream, [0u8; 1]))?;
    Ok(b[0])
}

async fn read_u16(stream: &mut TcpStream) -> io::Result<u16> {
    let (_s, b) = await!(read_exact(stream, [0u8; 2]))?;
    Ok(BigEndian::read_u16(&b))
}

pub async fn read_socks_address(mut stream: &mut TcpStream) -> Result<Address, SocksError> {
    let addr_type: u8 = await!(read_u8(stream))?;
    match addr_type.try_into()? {
        consts::AddrType::IPV4 => {
            let v4addr = Ipv4Addr::new(await!(read_u8(stream))?,
                                       await!(read_u8(stream))?,
                                       await!(read_u8(stream))?,
                                       await!(read_u8(stream))?);
            let port = await!(read_u16(stream))?;
            let addr = Address::SocketAddress(SocketAddr::V4(SocketAddrV4::new(v4addr, port)));
            Ok(addr)
        }
        consts::AddrType::IPV6 => {
            let v6addr = Ipv6Addr::new(await!(read_u16(stream))?,
                                       await!(read_u16(stream))?,
                                       await!(read_u16(stream))?,
                                       await!(read_u16(stream))?,
                                       await!(read_u16(stream))?,
                                       await!(read_u16(stream))?,
                                       await!(read_u16(stream))?,
                                       await!(read_u16(stream))?);
            let port = await!(read_u16(stream))?;

            let addr = Address::SocketAddress(SocketAddr::V6(SocketAddrV6::new(v6addr, port, 0, 0)));
            Ok(addr)
        }
        consts::AddrType::DomainName => {
            // domain and port
            let length = await!(read_u8(stream))?;
            let addr_len = length - 2;
            let mut raw_addr= vec![];
            raw_addr.resize(addr_len as usize, 0);
            await!(read_exact(&mut stream, &mut raw_addr))?;

            let addr = match String::from_utf8(raw_addr) {
                Ok(addr) => addr,
                Err(..) => return Err(SocksError::InvalidDomainEncoding.into()),
            };
            let port = await!(read_u16(stream))?;

            let addr = Address::DomainNameAddress(addr, port);
            Ok(addr)
        }
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
        TcpResponseHeader { reply: reply,
            address: address, }
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
pub async fn read_handshake_request(mut s: &mut TcpStream)
    -> Result<HandshakeRequest, Error>  {
    let (_s, buf) = await!(read_exact(&mut s, [0u8, 0u8]))?;
    let ver = buf[0];
    let nmet = buf[1];

    if ver != consts::SOCKS5_VERSION {
        s.shutdown(Shutdown::Both)?;
        return Err(SocksError::SocksVersionNoSupport {ver: ver}.into());
    }
    let (_s, methods) = await!(read_exact(&mut s, vec![0u8; nmet as usize]))?;
    Ok(HandshakeRequest { methods: methods })
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
