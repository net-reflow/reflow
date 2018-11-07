//! read some bytes and guess the type
use bytes::BytesMut;

mod codec;
mod parse;
pub use self::codec::parse_first_packet;
pub use self::parse::TcpProtocol;

pub struct InspectedTcp {
    /// bytes read
    pub bytes: BytesMut,
    #[allow(dead_code)]
    pub protocol: TcpProtocol,
}