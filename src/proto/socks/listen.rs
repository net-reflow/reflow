use failure::Error;
use tokio::net::TcpStream;

use super::read_handshake_request;
use super::heads::handle_socks_head;
use super::heads::read_command;
use crate::proto::socks::TcpRequestHeader;

pub async fn handle_socks_handshake(ts: TcpStream) -> Result<(TcpStream, TcpRequestHeader), Error> {
    let peer = ts.peer_addr()?;
    let (s, h) = await!(read_handshake_request(ts))?;
    let s = await!(handle_socks_head(s, h))?;
    await!(read_command(s, peer))
}

