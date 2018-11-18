use tokio::net::TcpStream;
use super::SocksError;
use crate::proto::socks::consts::Reply;
use std::convert::TryInto;
use tokio::io::read_exact;
use crate::proto::socks::read_socks_address;
use crate::proto::socks::Address;

/// response from a socks proxy server
async fn read_response_head(mut socket: &mut TcpStream) -> Result<Address, SocksError> {

    let (_s, b) = await!(read_exact(&mut socket, [0u8; 3]))?;
    let ver = b[0];
    let r = b[1];
    let res = b[2];
    if ver != 5 {
        let e = SocksError::SocksVersionNoSupport { ver };
        return Err(e);
    }

    let r: Reply = r.try_into()?;
    match r {
        Reply::SUCCEEDED => {},
        o => return Err(SocksError::RepliedError { reply: o}),
    }

    if res != 0 {
        return Err(SocksError::ProtocolIncorrect);
    }
    await!(read_socks_address(socket))
}
