use tokio::net::TcpStream;
use futures::Future;
use failure::Error;
use futures::future::Either;
use tokio_io::io::write_all;
use bytes::BufMut;

use proto::socks::consts::AuthMethod;
use proto::socks::SocksError;
use proto::socks::HandshakeRequest;
use bytes::BytesMut;
use proto::socks::consts;

pub fn handle_socks_head(s: TcpStream, h: HandshakeRequest) -> impl Future<Item=TcpStream, Error=Error> {
    trace!("socks req: {:?}", h);
    if !h.methods.contains(&(AuthMethod::NONE as u8)) {
        warn!("Currently does not support socks authentication");
        Either::A(write_socks_response(s, AuthMethod::NotAcceptable).then(|_r| {
            Err(SocksError::NoSupportAuth.into())
        }))
    } else {
        Either::B(write_socks_response(s, AuthMethod::NONE))
    }
}

fn write_socks_response(s: TcpStream, meth: AuthMethod) -> impl Future<Item=TcpStream, Error=Error> {
    let mut buf = BytesMut::with_capacity(2);
    buf.put_slice(&[consts::SOCKS5_VERSION as u8, meth as u8]);
    write_all(s, buf)
        .map(|(s, _b)| s)
        .map_err(|e| e.into())
}