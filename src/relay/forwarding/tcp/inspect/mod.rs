//! read some bytes and guess the type

use tokio::net::TcpStream;
use futures::Future;
use failure::Error;
use bytes::BytesMut;
use bytes::Bytes;

mod codec;
use self::codec::ReadGuessHead;

struct TcpTrafficInfo {
}

pub struct InspectedTcp {
    /// client stream
    pub stream: TcpStream,
    /// bytes read
    pub bytes: BytesMut,
    kind: TcpTrafficInfo,
}

/// read some bytes and guess
pub fn inspect_tcp(stream: TcpStream) -> impl Future<Item=InspectedTcp, Error=Error> {
    ReadGuessHead::new(stream)
}