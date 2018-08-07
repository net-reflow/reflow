//! implement header guessing

use tokio::net::TcpStream;
use tokio::io::AsyncRead;
use bytes::BytesMut;
use futures::Async;
use futures::Future;
use failure::Error;

use super::InspectedTcp;
use relay::inspect::parse::guess_bytes;

/// Read the first bytes from the client
/// combined with the SocketAddr already given
/// choose a route according to rules configured
pub struct ParseFirstPacket {
    state: Option<ReadGuessHeadState>,
}

struct  ReadGuessHeadState {
    socket: TcpStream,
    rd: BytesMut,
}

impl ParseFirstPacket {
    pub fn new(socket: TcpStream) -> Self {
        let buf = BytesMut::new();
        let s = ReadGuessHeadState {
            socket: socket,
            rd: buf,
        };
        ParseFirstPacket { state: Some(s) }
    }
}

impl Future for ParseFirstPacket {
    type Item = InspectedTcp;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        loop {
            // First, read some data
            match self.state {
                Some(ReadGuessHeadState{ ref mut socket, ref mut rd}) => {
                    rd.reserve(1024);
                    try_ready!(socket.read_buf(rd));
                }
                None => panic!("poll after finish"),
            }

            let guess = {
                match self.state {
                    Some(ReadGuessHeadState { socket: ref _s, ref rd }) => {
                        guess_bytes(&rd)
                    }
                    _ => panic!("poll after finish"),
                }
            };

            {
                trace!("detected protocol {:?}", guess);
                let s = self.state.take().unwrap();
                let socket = s.socket;
                let line = s.rd;

                let tcp = InspectedTcp {
                    stream: socket,
                    bytes: line,
                    protocol: guess,
                };
                // Return the line
                return Ok(Async::Ready(tcp));
            }
        }
    }
}