//! Socks5 protocol definition (RFC1928)
//!
//! Implements [SOCKS Protocol Version 5](https://www.ietf.org/rfc/rfc1928.txt) proxy protocol

use std::convert::From;
use std::convert::TryInto;
use std::fmt::{self, Debug, Formatter};
use std::io::{self, Cursor, Read};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::net::Shutdown;
use std::u8;

use byteorder::{BigEndian, ReadBytesExt};
use bytes::{Bytes, BytesMut, IntoBuf};

use failure::Error;
use futures::{Async, Future, Poll};

use tokio_io::io::read_exact;
use tokio_io::{AsyncRead};

mod consts;
pub mod listen;
mod heads;
mod codec;

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
    #[inline]
    pub fn read_from<R: AsyncRead>(stream: R) -> ReadAddress<R> {
        ReadAddress::new(stream)
    }

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

impl From<SocketAddr> for Address {
    fn from(s: SocketAddr) -> Address {
        Address::SocketAddress(s)
    }
}

impl From<(String, u16)> for Address {
    fn from((dn, port): (String, u16)) -> Address {
        Address::DomainNameAddress(dn, port)
    }
}

async fn read_u8<R: AsyncRead>(stream: R) -> io::Result<u8> {
    let (_s, b) = await!(read_exact(stream, [0u8; 1]))?;
    Ok(b[0])
}

async fn read_u16<R: AsyncRead>(stream: R) -> io::Result<u16> {
    let (_s, b) = await!(read_exact(stream, [0u8; 2]))?;
    Ok(BigEndian::read_u16(&b))
}

pub async fn read_socks_address<R: AsyncRead>(mut stream: R) -> Result<Address, Error> {
    let addr_type: u8 = await!(read_u8(&mut stream))?;
    match addr_type.try_into()? {
        consts::AddrType::IPV4 => {
            let v4addr = Ipv4Addr::new(await!(read_u8(&mut stream))?,
                                       await!(read_u8(&mut stream))?,
                                       await!(read_u8(&mut stream))?,
                                       await!(read_u8(&mut stream))?);
            let port = await!(read_u16(stream))?;
            let addr = Address::SocketAddress(SocketAddr::V4(SocketAddrV4::new(v4addr, port)));
            Ok(addr)
        }
        consts::AddrType::IPV6 => {
            let v6addr = Ipv6Addr::new(await!(read_u16(&mut stream))?,
                                       await!(read_u16(&mut stream))?,
                                       await!(read_u16(&mut stream))?,
                                       await!(read_u16(&mut stream))?,
                                       await!(read_u16(&mut stream))?,
                                       await!(read_u16(&mut stream))?,
                                       await!(read_u16(&mut stream))?,
                                       await!(read_u16(&mut stream))?);
            let port = await!(read_u16(stream))?;

            let addr = Address::SocketAddress(SocketAddr::V6(SocketAddrV6::new(v6addr, port, 0, 0)));
            Ok(addr)
        }
        consts::AddrType::DomainName => {
            // domain and port
            let length = await!(read_u8(&mut stream))?;
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

pub struct ReadAddress<R>
    where R: AsyncRead
{
    reader: Option<R>,
    state: ReadAddressState,
    buf: Option<BytesMut>,
    already_read: usize,
}

enum ReadAddressState {
    ReadingType,
    ReadingIpv4,
    ReadingIpv6,
    ReadingDomainNameLength,
    ReadingDomainName,
}

impl<R> Future for ReadAddress<R>
    where R: AsyncRead
{
    type Item = (R, Address);
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        debug_assert!(self.reader.is_some());

        loop {
            match self.state {
                ReadAddressState::ReadingType => {
                    try_ready!(self.read_addr_type());
                }
                ReadAddressState::ReadingIpv4 => {
                    let addr = try_ready!(self.read_ipv4());
                    let reader = self.reader.take().unwrap();
                    return Ok((reader, addr).into());
                }
                ReadAddressState::ReadingIpv6 => {
                    let addr = try_ready!(self.read_ipv6());
                    let reader = self.reader.take().unwrap();
                    return Ok((reader, addr).into());
                }
                ReadAddressState::ReadingDomainNameLength => {
                    try_ready!(self.read_domain_name_length());
                }
                ReadAddressState::ReadingDomainName => {
                    let addr = try_ready!(self.read_domain_name());
                    let reader = self.reader.take().unwrap();
                    return Ok((reader, addr).into());
                }
            };
        }
    }
}

impl<R> ReadAddress<R>
    where R: AsyncRead
{
    fn new(r: R) -> ReadAddress<R> {
        ReadAddress { reader: Some(r),
            state: ReadAddressState::ReadingType,
            buf: None,
            already_read: 0, }
    }

    fn read_addr_type(&mut self) -> Poll<(), Error> {
        let b = {
            let r = self.reader.as_mut().unwrap();
            r.read_u8()
        };
        let addr_type = try_nb!(b);
        match addr_type.try_into()? {
            consts::AddrType::IPV4 => {
                self.state = ReadAddressState::ReadingIpv4;
                self.alloc_buf(6);
            }
            consts::AddrType::IPV6 => {
                self.state = ReadAddressState::ReadingIpv6;
                self.alloc_buf(18);
            }
            consts::AddrType::DomainName => {
                self.state = ReadAddressState::ReadingDomainNameLength;
            }
        };

        Ok(Async::Ready(()))
    }

    fn read_ipv4(&mut self) -> Poll<Address, Error> {
        try_ready!(self.read_data());
        let mut stream: Cursor<Bytes> = self.freeze_buf().into_buf();
        let v4addr = Ipv4Addr::new(stream.read_u8()?,
                                   stream.read_u8()?,
                                   stream.read_u8()?,
                                   stream.read_u8()?);
        let port = stream.read_u16::<BigEndian>()?;
        let addr = Address::SocketAddress(SocketAddr::V4(SocketAddrV4::new(v4addr, port)));
        Ok(Async::Ready(addr))
    }

    fn read_ipv6(&mut self) -> Poll<Address, Error> {
        try_ready!(self.read_data());
        let mut stream: Cursor<Bytes> = self.freeze_buf().into_buf();
        let v6addr = Ipv6Addr::new(stream.read_u16::<BigEndian>()?,
                                   stream.read_u16::<BigEndian>()?,
                                   stream.read_u16::<BigEndian>()?,
                                   stream.read_u16::<BigEndian>()?,
                                   stream.read_u16::<BigEndian>()?,
                                   stream.read_u16::<BigEndian>()?,
                                   stream.read_u16::<BigEndian>()?,
                                   stream.read_u16::<BigEndian>()?);
        let port = stream.read_u16::<BigEndian>()?;

        let addr = Address::SocketAddress(SocketAddr::V6(SocketAddrV6::new(v6addr, port, 0, 0)));
        Ok(Async::Ready(addr))
    }

    fn read_domain_name_length(&mut self) -> Poll<(), Error> {
        let length = try_nb!(self.reader.as_mut().unwrap().read_u8());
        self.state = ReadAddressState::ReadingDomainName;
        self.alloc_buf(length as usize + 2);
        Ok(Async::Ready(()))
    }

    fn read_domain_name(&mut self) -> Poll<Address, Error> {
        try_ready!(self.read_data());
        let buf = self.freeze_buf();
        let addr_len = buf.len() - 2;
        let mut stream: Cursor<Bytes> = buf.into_buf();

        let mut raw_addr = Vec::with_capacity(addr_len);
        unsafe {
            raw_addr.set_len(addr_len);
        }
        stream.read_exact(&mut raw_addr)?;

        let addr = match String::from_utf8(raw_addr) {
            Ok(addr) => addr,
            Err(..) => return Err(SocksError::InvalidDomainEncoding.into()),
        };
        let port = stream.read_u16::<BigEndian>()?;

        let addr = Address::DomainNameAddress(addr, port);
        Ok(Async::Ready(addr))
    }

    fn alloc_buf(&mut self, size: usize) {
        let mut buf = BytesMut::with_capacity(size);
        unsafe {
            buf.set_len(size);
        }
        self.buf = Some(buf);
    }

    fn read_data(&mut self) -> Poll<(), io::Error> {
        let buf = self.buf.as_mut().unwrap();

        while self.already_read < buf.len() {
            match self.reader.as_mut().unwrap().read(&mut buf[self.already_read..]) {
                Ok(0) => {
                    let err = io::Error::new(io::ErrorKind::Other, "Unexpected EOF");
                    return Err(err);
                }
                Ok(n) => self.already_read += n,
                Err(err) => return Err(err),
            }
        }

        Ok(Async::Ready(()))
    }

    fn freeze_buf(&mut self) -> Bytes {
        let buf = self.buf.take().unwrap();
        buf.freeze()
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
pub fn read_handshake_request(s: TcpStream)
    -> impl Future<Item = (TcpStream, HandshakeRequest), Error = Error>  {
    read_exact(s, [0u8, 0u8])
        .map_err(|e| e.into())
        .and_then(|(r, buf)| {
        let ver = buf[0];
        let nmet = buf[1];

        if ver != consts::SOCKS5_VERSION {
            r.shutdown(Shutdown::Both)?;
            return Err(SocksError::SocksVersionNoSupport {ver: ver}.into());
        }

        Ok((r, nmet))
    })
        .and_then(|(r, nmet)| {
            read_exact(r, vec![0u8; nmet as usize]).map_err(|e| e.into())
        })
        .and_then(|(r, methods)| Ok((r, HandshakeRequest { methods: methods })))
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
