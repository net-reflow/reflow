//! read some bytes and guess the type

use tokio::net::TcpStream;
use futures::Future;
use failure::Error;
use bytes::BytesMut;

mod codec;
mod parse;
pub use self::codec::RouteByHeader;
pub use self::parse::TcpTrafficInfo;
use super::super::routing::RoutingDecision;

pub struct InspectedTcp {
    /// client stream
    pub stream: TcpStream,
    /// bytes read
    pub bytes: BytesMut,
    #[allow(dead_code)]
    route: RoutingDecision,
}