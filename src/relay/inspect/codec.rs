//! implement header guessing

use bytes::BytesMut;
use failure::Error;
use tokio::net::TcpStream;

use futures::compat::Future01CompatExt;
use tokio_io::io::read;

use super::InspectedTcp;
use crate::relay::inspect::parse::guess_bytes;

#[allow(unused_assignments)]
pub async fn parse_first_packet(socket: &mut TcpStream) -> Result<InspectedTcp, Error> {
    let mut buf = BytesMut::new();
    buf.resize(1024, 0);
    let mut pos = 0;
    let socket = socket;
    // FIXME: in effect, it only reads the first packet
    while pos < buf.len() {
        let (_socket, _, bs) = await!(read(socket, &mut buf[pos..]).compat())?;
        //let bs = await!(socket.read_async(&mut buf[pos..]))?;
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
