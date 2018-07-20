use tokio::net::TcpStream;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio_io::io::copy;
use tokio_io::io::write_all;
use tokio;
use futures::Future;
use failure::Error;
use std::net::SocketAddr;

mod inspect;
use self::inspect::ParseFirstPacket;
pub use self::inspect::TcpProtocol;
use std::sync::Arc;
use relay::TcpRouter;

pub fn handle_incoming_tcp(client_stream: TcpStream, a: SocketAddr, router: Arc<TcpRouter>)-> impl Future<Item=(), Error=Error> {
    ParseFirstPacket::new(client_stream, a, router).and_then(move|tcp| {
        let client_stream = tcp.stream;
        let data = tcp.bytes;
        let s = tokio::net::TcpStream::connect(&a).map_err(|e| e.into());
        s.and_then(|stream| write_all(stream, data).map_err(|e| e.into()))
            .and_then(|(stream, _)| {
            let (ur, uw) = stream.split();
            let (cr, cw) = client_stream.split();
            tokio::spawn(run_copy(ur, cw));
            tokio::spawn(run_copy(cr, uw));
            Ok(())
        })
    }).map_err(|e| e.into())
}


fn run_copy<R, W>(reader: R, writer: W) -> impl Future<Item=(), Error=()>
    where R: AsyncRead,
          W: AsyncWrite, {
    copy(reader, writer).map(|_x| ())
        .map_err(|e| error!("Error copying {:?}", e))
}

