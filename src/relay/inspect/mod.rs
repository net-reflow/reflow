//! read some bytes and guess the type

use tokio::net::TcpStream;
use bytes::BytesMut;

mod codec;
mod parse;
pub use self::codec::ParseFirstPacket;
pub use self::parse::TcpProtocol;

pub struct InspectedTcp {
    /// client stream
    pub stream: TcpStream,
    /// bytes read
    pub bytes: BytesMut,
    #[allow(dead_code)]
    pub protocol: TcpProtocol,
}