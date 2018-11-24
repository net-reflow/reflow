use tokio::net::TcpStream;
use super::SocksError;
use crate::proto::socks::consts::Reply;
use std::convert::TryInto;
use std::io;
use tokio::io::{read_exact, write_all};
use crate::proto::socks::read_socks_address;
use super::consts::{SOCKS5_VERSION, AuthMethod};
use std::net::Shutdown;
use crate::proto::socks::HandshakeResponse;
use bytes::{BytesMut, BufMut};
use crate::proto::socks::consts;
use super::Address;
use crate::proto::socks::codec::write_address;

pub mod udp;

/// response from a socks proxy server
async fn read_response_head(mut socket: &mut TcpStream) -> Result<Address, SocksError> {

    let (_s, b) = await!(read_exact(&mut socket, [0u8; 3]))?;
    let ver = b[0];
    let r = b[1];
    let res = b[2];
    if ver != 5 {
        let e = SocksError::SocksVersionNoSupport { ver };
        return Err(e);
    }

    let r: Reply = r.try_into()?;
    match r {
        Reply::SUCCEEDED => {},
        o => return Err(SocksError::RepliedError { reply: o}),
    }

    if res != 0 {
        return Err(SocksError::InvalidData { msg: "Reserved byte not zero", data: vec![res]});
    }
    await!(read_socks_address(socket))
}

/// given a stream already connected to a socks server without auth
/// instruct it to connect to a target
pub async fn connect_socks_to(mut stream: &mut TcpStream, target: Address)
                              -> Result<Address, SocksError> {
    await!(connect_socks_command(&mut stream, target, consts::Command::TcpConnect))
}

pub async fn connect_socks_udp(mut stream: &mut TcpStream, target: Address)
                               -> Result<Address, SocksError> {
    await!(connect_socks_command(&mut stream, target, consts::Command::UdpAssociate))
}

async fn connect_socks_command(
    mut stream: &mut TcpStream,
    target: Address,
    cmd: consts::Command,
) -> Result<Address, SocksError> {

    let packet = [SOCKS5_VERSION,
        1, // one method
        0, // no auth
    ];
    await!(write_all(&mut stream, &packet))?;
    await!(write_command_request(&mut stream, target, cmd))?;

    await!(read_handshake_response(&mut stream))?;
    let a = await!(read_response_head(&mut stream))?;
    Ok(a)
}

/// make sure version and auth are as expected
async fn read_handshake_response(mut s: &mut TcpStream)
                                    -> Result<HandshakeResponse, SocksError>  {
    let (_s, buf) = await!(read_exact(&mut s, [0u8, 0u8]))?;
    let ver = buf[0];
    let cmet = buf[1];

    if ver != SOCKS5_VERSION {
        s.shutdown(Shutdown::Both)?;
        return Err(SocksError::SocksVersionNoSupport {ver: ver});
    }
    if cmet != AuthMethod::NONE as u8 {
        return Err(SocksError::NoSupportAuth);
    }
    Ok(HandshakeResponse { chosen_method: cmet })
}

async fn write_command_request(s: &mut TcpStream, addr: Address, cmd: consts::Command)
                                      -> Result<(), io::Error> {
    let mut buf = BytesMut::with_capacity((&addr).len());
    buf.put_slice(&[SOCKS5_VERSION,
        cmd as u8,
        0x00, // reserved
    ]);
    write_address(&addr, &mut buf);
    await!(write_all(s, buf))?;
    Ok(())
}