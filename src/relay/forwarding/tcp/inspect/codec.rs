//! implement header guessing

use tokio::net::TcpStream;
use tokio::io::AsyncRead;
use bytes::BytesMut;
use futures::Async;
use std::mem;
use futures::Future;
use failure::Error;

use super::InspectedTcp;
use relay::forwarding::tcp::inspect::parse::guess_bytes;

pub struct ReadGuessHead {
    state: ReadGuessHeadState
}

enum ReadGuessHeadState {
    Reading {
        socket: TcpStream,
        rd: BytesMut,
    },
    Finished,
}

impl ReadGuessHead {
    pub fn new(socket: TcpStream) -> Self {
        let buf = BytesMut::new();
        let s = ReadGuessHeadState::Reading {
            socket: socket,
            rd: buf,
        };
        ReadGuessHead { state: s}
    }
}

impl Future for ReadGuessHead {
    type Item = InspectedTcp;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        loop {
            // First, read some data
            let n = {
                let (mut socket, mut rd) = match self.state {
                    ReadGuessHeadState::Reading {ref mut socket, ref mut rd} => (socket, rd),
                    _ => panic!("poll after finish"),
                };
                rd.reserve(1024);
                try_ready!(socket.read_buf(&mut rd))
            };

            let guess = {
                match self.state {
                    ReadGuessHeadState::Reading {socket: ref _s, ref rd} => {
                        guess_bytes(&rd)
                    }
                    _ => panic!("poll after finish"),
                }
            };

            if let Some(info) = guess {
                debug!("guessed {:?}", info);
                let s = mem::replace(&mut self.state, ReadGuessHeadState::Finished);
                let (socket, rd) = match s {
                    ReadGuessHeadState::Reading {socket, rd} => (socket, rd),
                    _ => panic!("poll after finish"),
                };
                let line = rd;

                let tcp = InspectedTcp {
                    stream: socket,
                    bytes: line,
                    kind: info,
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