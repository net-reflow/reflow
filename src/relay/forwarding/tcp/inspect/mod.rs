//! read some bytes and guess the type

use tokio::net::TcpStream;
use bytes::BytesMut;

mod codec;
mod parse;
pub use self::codec::ParseFirstPacket;
pub use self::parse::TcpProtocol;
use super::super::routing::RoutingDecision;
use std::sync::Arc;

pub struct InspectedTcp {
    /// client stream
    pub stream: TcpStream,
    /// bytes read
    pub bytes: BytesMut,
    #[allow(dead_code)]
    pub protocol: TcpProtocol,
}