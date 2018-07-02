use tokio;

use std::net::SocketAddr;
use failure::Error;
use futures::Stream;
use futures::Future;
use futures::future;
use tokio::net::TcpStream;

use super::SocksError;
use super::read_handshake_request;
use super::consts::{AuthMethod};
use super::heads::handle_socks_head;
use super::heads::read_command;
use std::io;

pub fn listen(addr: &SocketAddr)
    ->Result<impl Stream<Item=impl Future<Item=TcpStream,Error=Error>, Error=io::Error>, io::Error> {
    let l = tokio::net::TcpListener::bind(addr)?;
    let f =
        l.incoming().map_err(|e| e.into())
            .and_then(|s| {
        let peer = s.peer_addr()?;
        let f = handle_handshake(s, peer);
        Ok(f)
    });
    Ok(f)
}

fn handle_handshake(ts: TcpStream, peer: SocketAddr) -> impl Future<Item=TcpStream, Error=Error> {
    read_handshake_request(ts).and_then(|(s, h)| {
        handle_socks_head(s, h)
    })
        .and_then(move|s: TcpStream| {
            read_command(s, peer).map(|(s,h)| {
                debug!("command: {:?}", h);
                s
            })
        })
}