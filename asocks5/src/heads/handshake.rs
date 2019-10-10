use log::{trace, warn};
use tokio::net::TcpStream;

use crate::consts;
use crate::consts::AuthMethod;
use crate::socks::HandshakeRequest;
use crate::socks::SocksError;
use tokio::prelude::*;

pub async fn handle_socks_head(s: &mut TcpStream, h: HandshakeRequest) -> Result<(), SocksError> {
    trace!("socks req: {:?}", h);
    if !h.methods.contains(&(AuthMethod::NONE as u8)) {
        warn!("Currently does not support socks authentication");
        write_socks_response(s, AuthMethod::NotAcceptable).await?;
        return Err(SocksError::NoSupportAuth);
    } else {
        write_socks_response(s, AuthMethod::NONE).await?;
    }
    Ok(())
}

async fn write_socks_response(s: &mut TcpStream, meth: AuthMethod) -> Result<(), SocksError> {
    let buf = &[consts::SOCKS5_VERSION as u8, meth as u8];
    s.write_all(buf).await?;
    Ok(())
}
