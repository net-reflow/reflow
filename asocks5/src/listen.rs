use failure::Error;
use tokio::net::TcpStream;

use super::socks::read_handshake_request;
use crate::heads::handle_socks_head;
use crate::heads::read_command_async;
use crate::socks::TcpRequestHeader;

pub async fn handle_socks_handshake(
    mut ts: TcpStream,
) -> Result<(TcpStream, TcpRequestHeader), Error> {
    let peer = ts.peer_addr()?;
    let h = await!(read_handshake_request(&mut ts))?;
    await!(handle_socks_head(&mut ts, h))?;
    let (s, req) = await!(read_command_async(ts, peer))?;
    Ok((s, req))
}
