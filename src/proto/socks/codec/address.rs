use bytes::BufMut;
use std::net::SocketAddrV4;
use std::io::Cursor;
use std::io::Write;
use byteorder::BigEndian;
use byteorder::WriteBytesExt;
use byteorder::ReadBytesExt;
use std::net::SocketAddrV6;
use std::net::SocketAddr;
use crate::proto::socks::Address;
use crate::proto::socks::consts;
use std::io::Read;
use crate::proto::socks::SocksError;
use std::convert::TryInto;
use std::net::{Ipv4Addr, Ipv6Addr};

pub fn write_address<B: BufMut>(addr: &Address, buf: &mut B) {
    match *addr {
        Address::SocketAddress(sa) => {
            match sa {
                SocketAddr::V4(ref addr) => write_ipv4_address(addr, buf),
                SocketAddr::V6(ref addr) => write_ipv6_address(addr, buf),
            }
        }
        Address::DomainNameAddress(ref dnaddr, ref port) => write_domain_name_address(dnaddr, *port, buf),
    }
}

fn write_ipv4_address<B: BufMut>(addr: &SocketAddrV4, buf: &mut B) {
    let mut dbuf = [0u8; 1 + 4 + 2];
    {
        let mut cur = Cursor::new(&mut dbuf[..]);
        let _ = cur.write_u8(consts::AddrType::IPV4 as u8); // Address type
        let _ = cur.write_all(&addr.ip().octets()); // Ipv4 bytes
        let _ = cur.write_u16::<BigEndian>(addr.port());
    }
    buf.put_slice(&dbuf[..]);
}

fn write_ipv6_address<B: BufMut>(addr: &SocketAddrV6, buf: &mut B) {
    let mut dbuf = [0u8; 1 + 16 + 2];

    {
        let mut cur = Cursor::new(&mut dbuf[..]);
        let _ = cur.write_u8(consts::AddrType::IPV6 as u8); // Address type
        for seg in &addr.ip().segments() {
            let _ = cur.write_u16::<BigEndian>(*seg);
        }
        let _ = cur.write_u16::<BigEndian>(addr.port());
    }

    buf.put_slice(&dbuf[..]);
}

fn write_domain_name_address<B: BufMut>(dnaddr: &str, port: u16, buf: &mut B) {
    assert!(dnaddr.len() <= u8::max_value() as usize);

    buf.put_u8(consts::AddrType::DomainName as u8);
    buf.put_u8(dnaddr.len() as u8);
    buf.put_slice(dnaddr[..].as_bytes());
    buf.put_u16_be(port);
}

pub fn read_address<R: Read>(stream: &mut R) -> Result<Address, SocksError> {
    let mut b = [0u8; 1];
    stream.read_exact( &mut b)?;
    let addr_type: consts::AddrType = b[0].try_into()?;
    match addr_type {
        consts::AddrType::IPV4 => {
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf)?;
            let v4addr = Ipv4Addr::from(stream.read_u32::<BigEndian>()?);
            let port = stream.read_u16::<BigEndian>()?;
            let addr = Address::SocketAddress(SocketAddr::V4(SocketAddrV4::new(v4addr, port)));
            Ok(addr)
        }
        consts::AddrType::IPV6 => {
            let mut buf = [0u8; 16];
            stream.read_exact(&mut buf)?;
            let v6addr = Ipv6Addr::from(buf);
            let mut buf = [0u8; 2];
            stream.read_exact(&mut buf)?;
            let port = stream.read_u16::<BigEndian>()?;

            let addr = Address::SocketAddress(SocketAddr::V6(SocketAddrV6::new(v6addr, port, 0, 0)));
            Ok(addr)
        }
        consts::AddrType::DomainName => {
            let mut b = [0u8; 1];
            stream.read_exact(&mut b)?;
            let length = b[0] as usize;
            let addr_len = length - 2;
            let mut raw_addr= vec![];
            raw_addr.resize(addr_len, 0);
            stream.read_exact(&mut raw_addr)?;
            let port = stream.read_u16::<BigEndian>()?;
            let addr = match String::from_utf8(raw_addr) {
                Ok(addr) => addr,
                Err(..) => return Err(SocksError::InvalidDomainEncoding),
            };
            let addr = Address::DomainNameAddress(addr, port);
            Ok(addr)
        }
    }
}

