use tokio::net::TcpStream;
use proto::socks::TcpRequestHeader;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio_io::io::copy;
use tokio;
use futures::Future;
use failure::Error;
use std::net::SocketAddr;

pub fn handle_incoming_tcp(client_stream: TcpStream, a: &SocketAddr)-> impl Future<Item=(), Error=Error> {
    let s = tokio::net::TcpStream::connect(&a);
    let f = s.and_then(|stream| {
        let (ur, uw) = stream.split();
        let (cr, cw) = client_stream.split();
        tokio::spawn(run_copy(ur, cw));
        tokio::spawn(run_copy(cr, uw));
        Ok(())
    }).map_err(|e| e.into());
    f
}

fn run_copy<R, W>(reader: R, writer: W) -> impl Future<Item=(), Error=()>
    where R: AsyncRead,
          W: AsyncWrite, {
    copy(reader, writer).map(|_x| ())
        .map_err(|e| error!("Error copying {:?}", e))
}

