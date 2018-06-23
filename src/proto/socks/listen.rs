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

pub fn listen(addr: &SocketAddr)->Result<(), Error >{
    let l = tokio::net::TcpListener::bind(addr)?;
    let f = l.incoming().for_each(|s| {
        tokio::spawn(handle_client(s));
        Ok(())
    }).map_err(|e| {
        error!("Listen error {:?}", e);
    });
    tokio::spawn(f);
    Ok(())
}

fn handle_client(ts: TcpStream) -> impl Future<Item=(), Error=()> {
    read_handshake_request(ts).and_then(|(s, h)| {
        handle_socks_head(s, h).map(|s| ())
    }).map_err(|e| error!("error handling client {:?}", e))
}