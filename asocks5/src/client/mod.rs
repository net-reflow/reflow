use super::consts::{AuthMethod, SOCKS5_VERSION};
use super::socks::SocksError;
use crate::codec::write_address;
use crate::codec::write_address_sa;
use crate::consts;
use crate::consts::Reply;
use crate::socks::read_socks_address;
use crate::socks::read_socks_socket_addr;
use crate::socks::Address;
use crate::socks::HandshakeResponse;
use bytes::{BufMut, Bytes, BytesMut};
use std::convert::TryInto;
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::prelude::*;

pub mod udp;

/// response from a socks proxy server
async fn read_response_head(socket: &mut TcpStream) -> Result<Address, SocksError> {
    read_response_head_3b(socket).await?;
    read_socks_address(socket).await
}

/// version, reply code, reserved
async fn read_response_head_3b(mut socket: &mut TcpStream) -> Result<(), SocksError> {
    let mut b = [0u8; 3];
    socket.read_exact(&mut b).await?;
    let ver = b[0];
    let r = b[1];
    let res = b[2];
    if ver != 5 {
        let e = SocksError::SocksVersionNoSupport { ver };
        return Err(e);
    }

    let r: Reply = r.try_into()?;
    match r {
        Reply::SUCCEEDED => {}
        o => return Err(SocksError::RepliedError { reply: o }),
    }

    if res != 0 {
        return Err(SocksError::InvalidData {
            msg: "Reserved byte not zero",
            data: vec![res],
        });
    }
    Ok(())
}

/// given a stream already connected to a socks server without auth
/// instruct it to connect to a target
pub async fn connect_socks_to(
    mut stream: &mut TcpStream,
    target: Address,
) -> Result<Address, SocksError> {
    connect_socks_command(&mut stream, target, consts::Command::TcpConnect).await
}

// avoid Address to avoid ICE
pub async fn connect_socks_socket_addr(
    mut stream: &mut TcpStream,
    target: SocketAddr,
) -> Result<SocketAddr, SocksError> {
    connect_socks_command_sa(&mut stream, target, consts::Command::TcpConnect).await
}

pub async fn connect_socks_udp(
    mut stream: &mut TcpStream,
    target: Address,
) -> Result<Address, SocksError> {
    connect_socks_command(&mut stream, target, consts::Command::UdpAssociate).await
}

async fn connect_socks_command(
    mut stream: &mut TcpStream,
    target: Address,
    cmd: consts::Command,
) -> Result<Address, SocksError> {
    let packet = [
        SOCKS5_VERSION,
        1, // one method
        0, // no auth
    ];
    stream.write_all(&packet).await?;
    read_handshake_response(&mut stream).await?;

    write_command_request(&mut stream, target, cmd).await?;

    let a = read_response_head(&mut stream).await?;
    Ok(a)
}

async fn connect_socks_command_sa(
    mut stream: &mut TcpStream,
    target: SocketAddr,
    cmd: consts::Command,
) -> Result<SocketAddr, SocksError> {
    let packet = [
        SOCKS5_VERSION,
        1, // one method
        0, // no auth
    ];
    stream.write_all(&packet).await?;
    read_handshake_response(&mut stream).await?;

    write_command_request_sa(&mut stream, target, cmd).await?;

    read_response_head_3b(&mut stream).await?;
    let a = read_socks_socket_addr(&mut stream).await?;
    Ok(a)
}

/// make sure version and auth are as expected
async fn read_handshake_response(mut s: &mut TcpStream) -> Result<HandshakeResponse, SocksError> {
    let mut buf = [0u8, 0u8];
    s.read_exact(&mut buf).await?;
    let ver = buf[0];
    let cmet = buf[1];

    if ver != SOCKS5_VERSION {
        s.shutdown().await?;
        return Err(SocksError::SocksVersionNoSupport { ver });
    }
    if cmet != AuthMethod::NONE as u8 {
        return Err(SocksError::NoSupportAuth);
    }
    Ok(HandshakeResponse {
        chosen_method: cmet,
    })
}

async fn write_command_request(
    s: &mut TcpStream,
    addr: Address,
    cmd: consts::Command,
) -> Result<(), io::Error> {
    let mut buf = BytesMut::with_capacity((&addr).len());
    buf.put_slice(&[
        SOCKS5_VERSION,
        cmd as u8,
        0x00, // reserved
    ]);
    write_address(&addr, &mut buf);
    s.write_all(buf.as_ref()).await?;
    Ok(())
}

async fn write_command_request_sa(
    s: &mut TcpStream,
    addr: SocketAddr,
    cmd: consts::Command,
) -> Result<(), io::Error> {
    let buf = command_request_packet(addr, cmd);
    s.write_all(buf.as_ref()).await?;
    Ok(())
}

fn command_request_packet(addr: SocketAddr, cmd: consts::Command) -> Bytes {
    let len = match addr {
        SocketAddr::V4(..) => 1 + 4 + 2,
        SocketAddr::V6(..) => 1 + 8 * 2 + 2,
    };
    let mut buf = BytesMut::with_capacity(len);
    buf.put_slice(&[
        SOCKS5_VERSION,
        cmd as u8,
        0x00, // reserved
    ]);
    write_address_sa(&addr, &mut buf);
    buf.freeze()
}
