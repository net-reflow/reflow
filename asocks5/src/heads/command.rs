use std::convert::TryInto;

use super::super::codec::write_address;
use crate::consts;
use crate::consts::Reply;
use crate::socks::read_socks_address;
use crate::socks::Address;
use crate::socks::SocksError;
use crate::socks::TcpRequestHeader;
use crate::socks::TcpResponseHeader;
use bytes::BufMut;
use bytes::BytesMut;
use futures_util::io::AsyncReadExt;
use futures_util::io::AsyncWriteExt;
use std::net::Shutdown;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::prelude::*;

pub async fn read_command_async(
    mut s: TcpStream,
    peer: SocketAddr,
) -> Result<(TcpStream, TcpRequestHeader), SocksError> {
    let mut buf = [0u8; 3];
    s.read_exact(&mut buf).await?;
    let ver = buf[0];
    if ver != consts::SOCKS5_VERSION {
        write_command_response_async(&mut s, Reply::ConnectionRefused, peer).await?;
        s.shutdown(Shutdown::Both)?;
        return Err(SocksError::SocksVersionNoSupport { ver });
    }
    let cmd = buf[1].try_into();
    if let Err(e) = cmd {
        let e: SocksError = e;
        write_command_response_async(&mut s, Reply::CommandNotSupported, peer).await?;
        s.shutdown(Shutdown::Both)?;
        return Err(e);
    }
    let cmd = cmd.unwrap();
    write_command_response_async(&mut s, Reply::SUCCEEDED, peer).await?;
    let address = read_socks_address(&mut s).await?;
    let header = TcpRequestHeader {
        command: cmd,
        address,
    };
    Ok((s, header))
}

async fn write_command_response_async(
    s: &mut TcpStream,
    rep: Reply,
    addr: SocketAddr,
) -> Result<(), SocksError> {
    let addr = Address::SocketAddress(addr);
    let resp = TcpResponseHeader::new(rep, addr);
    let mut buf = BytesMut::with_capacity(resp.len());
    buf.put_slice(&[consts::SOCKS5_VERSION, resp.reply as u8, 0x00]);
    write_address(&resp.address, &mut buf);
    s.write_all(buf.as_ref()).await?;
    Ok(())
}
