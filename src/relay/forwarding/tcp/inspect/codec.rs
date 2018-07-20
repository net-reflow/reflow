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
use relay::forwarding::tcp::inspect::parse::guess_bytes;
use std::sync::Arc;
use relay::TcpRouter;
use relay::forwarding::tcp::TcpProtocol;

/// Read the first bytes from the client
/// combined with the SocketAddr already given
/// choose a route according to rules configured
pub struct ParseFirstPacket {
    state: ReadGuessHeadState
}

enum ReadGuessHeadState {
    Reading {
        addr: SocketAddr,
        socket: TcpStream,
        rd: BytesMut,
        router: Arc<TcpRouter>,
    },
    Finished,
}

impl ParseFirstPacket {
    pub fn new(socket: TcpStream, addr: SocketAddr, router: Arc<TcpRouter>) -> Self {
        let buf = BytesMut::new();
        let s = ReadGuessHeadState::Reading {
            addr,
            socket: socket,
            rd: buf,
            router,
        };
        ParseFirstPacket { state: s}
    }
}

impl Future for ParseFirstPacket {
    type Item = InspectedTcp;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        loop {
            // First, read some data
            let n = {
                let (mut socket, mut rd) = match self.state {
                    ReadGuessHeadState::Reading {
                        addr: ref _a, ref mut socket, ref mut rd,
                        router: ref _r,
                    } => (socket, rd),
                    _ => panic!("poll after finish"),
                };
                rd.reserve(1024);
                try_ready!(socket.read_buf(&mut rd))
            };

            let guess = {
                match self.state {
                    ReadGuessHeadState::Reading {addr, socket: ref _s, ref rd, ref router} => {
                        guess_bytes(&rd)
                    }
                    _ => panic!("poll after finish"),
                }
            };

            {
                debug!("detected protocol {:?}", guess);
                let s = mem::replace(&mut self.state, ReadGuessHeadState::Finished);
                let (socket, rd) = match s {
                    ReadGuessHeadState::Reading {addr: _a, socket, rd, router: _r,} => (socket, rd),
                    _ => panic!("poll after finish"),
                };
                let line = rd;

                let tcp = InspectedTcp {
                    stream: socket,
                    bytes: line,
                    protocol: guess,
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