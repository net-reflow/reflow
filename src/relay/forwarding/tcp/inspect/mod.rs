//! read some bytes and guess the type

use tokio::net::TcpStream;
use futures::Future;
use failure::Error;
use bytes::BytesMut;

mod codec;
mod parse;
use self::codec::ReadGuessHead;

#[derive(Debug)]
pub enum  TcpTrafficInfo {
    PlainHttp(HttpInfo),
    SSH,
    Unidentified,
}

#[derive(Debug)]
pub struct HttpInfo {
    host: String,
    user_agent: Option<String>,
}

impl HttpInfo {
    pub fn new(h: &[u8], ua: Option<&[u8]>)-> HttpInfo {
        HttpInfo {
            host: String::from_utf8_lossy(h).into(),
            user_agent: ua.map(|b| String::from_utf8_lossy(b).into(),),
        }
    }
}

pub struct InspectedTcp {
    /// client stream
    pub stream: TcpStream,
    /// bytes read
    pub bytes: BytesMut,
    #[allow(dead_code)]
    kind: TcpTrafficInfo,
}

/// read some bytes and guess
pub fn inspect_tcp(stream: TcpStream) -> impl Future<Item=InspectedTcp, Error=Error> {
    ReadGuessHead::new(stream)
}