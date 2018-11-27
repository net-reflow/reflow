//! implement header guessing

use tokio::net::TcpStream;
use bytes::BytesMut;
use failure::Error;
use tokio::prelude::*;

use super::InspectedTcp;
use crate::relay::inspect::parse::guess_bytes;

#[allow(unused_assignments)]
pub async fn parse_first_packet(socket: &mut TcpStream)-> Result<InspectedTcp, Error> {
    let mut buf = BytesMut::new();
    buf.resize(1024, 0);
    let mut pos = 0;
    // FIXME: in effect, it only reads the first packet
    while pos < buf.len() {
        let bs = await!(socket.read_async(&mut buf[pos..]))?;
        pos += bs;
        let guess = guess_bytes(&buf);
        trace!("detected protocol {:?}", guess);

        buf.resize(pos, 0);
        let tcp = InspectedTcp {
            bytes: buf,
            protocol: guess,
        };
        return Ok(tcp);
    }
    Err(format_err!("Unidentified protocol"))
}