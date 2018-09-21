//! read some bytes and guess the type

use tokio::net::TcpStream;
use futures::Future;
use failure::Error;
use bytes::BytesMut;

mod codec;
mod parse;
use self::codec::ReadGuessHead;
pub use self::parse::TcpTrafficInfo;

pub struct InspectedTcp {
    /// client stream
    pub stream: TcpStream,
    /// bytes read
    pub bytes: BytesMut,
    #[allow(dead_code)]
    kind: String,
}

/// read some bytes and guess
pub fn inspect_tcp(stream: TcpStream) -> impl Future<Item=InspectedTcp, Error=Error> {
    ReadGuessHead::new(stream)
}