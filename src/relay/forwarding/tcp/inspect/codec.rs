//! implement header guessing

use tokio::net::TcpStream;
use tokio::io::AsyncRead;
use bytes::BytesMut;
use futures::Async;
use std::mem;
use futures::Future;
use failure::Error;
use std::net::SocketAddr;

use super::InspectedTcp;
use relay::forwarding::tcp::inspect::parse::route;

/// Read the first bytes from the client
/// combined with the SocketAddr already given
/// choose a route according to rules configured
pub struct RouteByHeader {
    state: ReadGuessHeadState
}

enum ReadGuessHeadState {
    Reading {
        addr: SocketAddr,
        socket: TcpStream,
        rd: BytesMut,
    },
    Finished,
}

impl RouteByHeader {
    pub fn new(socket: TcpStream, addr: SocketAddr) -> Self {
        let buf = BytesMut::new();
        let s = ReadGuessHeadState::Reading {
            addr,
            socket: socket,
            rd: buf,
        };
        RouteByHeader { state: s}
    }
}

impl Future for RouteByHeader {
    type Item = InspectedTcp;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        loop {
            // First, read some data
            let n = {
                let (mut socket, mut rd) = match self.state {
                    ReadGuessHeadState::Reading {addr: ref _a, ref mut socket, ref mut rd} => (socket, rd),
                    _ => panic!("poll after finish"),
                };
                rd.reserve(1024);
                try_ready!(socket.read_buf(&mut rd))
            };

            let guess = {
                match self.state {
                    ReadGuessHeadState::Reading {addr, socket: ref _s, ref rd} => {
                        route(&rd, addr)
                    }
                    _ => panic!("poll after finish"),
                }
            };

            if let Some(info) = guess {
                debug!("guessed {:?}", info);
                let s = mem::replace(&mut self.state, ReadGuessHeadState::Finished);
                let (socket, rd) = match s {
                    ReadGuessHeadState::Reading {addr: _a, socket, rd} => (socket, rd),
                    _ => panic!("poll after finish"),
                };
                let line = rd;

                let tcp = InspectedTcp {
                    stream: socket,
                    bytes: line,
                    route: info,
                };
                // Return the line
                return Ok(Async::Ready(tcp));
            }

            // if a match is not found by the end
            if n == 0 {
                return Err(format_err!("Client closed sock with incomplete head"));
            }
        }
    }
}