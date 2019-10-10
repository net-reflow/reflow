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
    let h = read_handshake_request(&mut ts).await?;
    handle_socks_head(&mut ts, h).await?;
    let (s, req) = read_command_async(ts, peer).await?;
    Ok((s, req))
}
