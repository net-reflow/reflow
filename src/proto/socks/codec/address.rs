use bytes::BufMut;
use std::net::SocketAddrV4;
use std::io::Cursor;
use std::io::Write;
use byteorder::BigEndian;
use byteorder::WriteBytesExt;
use std::net::SocketAddrV6;
use std::net::SocketAddr;
use proto::socks::Address;
use proto::socks::consts;

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


