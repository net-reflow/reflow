use tokio::net::TcpStream;
use tokio_io::io::write_all;

use crate::proto::socks::consts::AuthMethod;
use crate::proto::socks::SocksError;
use crate::proto::socks::HandshakeRequest;
use crate::proto::socks::consts;

pub async fn handle_socks_head(s: &mut TcpStream, h: HandshakeRequest)
    ->Result<(), SocksError> {
    trace!("socks req: {:?}", h);
    if !h.methods.contains(&(AuthMethod::NONE as u8)) {
        warn!("Currently does not support socks authentication");
            await!(write_socks_response(s, AuthMethod::NotAcceptable))?;
            return Err(SocksError::NoSupportAuth);
    } else {
        await!(write_socks_response(s, AuthMethod::NONE))?;
    }
    Ok(())
}

async fn write_socks_response(s: &mut TcpStream, meth: AuthMethod)
    -> Result<(), SocksError> {
    let buf = &[consts::SOCKS5_VERSION as u8, meth as u8];
    let (_s, _b) = await!(write_all(s, buf))?;
    Ok(())
}