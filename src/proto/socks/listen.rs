use failure::Error;
use tokio::net::TcpStream;

use super::read_handshake_request;
use super::heads::handle_socks_head;
use super::heads::read_command_async;
use crate::proto::socks::TcpRequestHeader;

pub async fn handle_socks_handshake(mut ts: TcpStream) -> Result<(TcpStream, TcpRequestHeader), Error> {
    let peer = ts.peer_addr()?;
    let h = await!(read_handshake_request(&mut ts))?;
    await!(handle_socks_head(&mut ts, h))?;
    await!(read_command_async(ts, peer))
}

