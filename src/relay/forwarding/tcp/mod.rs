use tokio::net::TcpStream;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio_io::io::copy;
use tokio_io::io::write_all;
use tokio;
use futures::Future;
use futures::future;
use futures::future::Either;
use failure::Error;
use std::net::SocketAddr;

mod inspect;
use self::inspect::ParseFirstPacket;
pub use self::inspect::TcpProtocol;
use std::sync::Arc;
use relay::TcpRouter;
use bytes::Bytes;
use relay::forwarding::routing::RoutingDecision;
use relay::forwarding::routing::Route;

pub fn handle_incoming_tcp(client_stream: TcpStream, a: SocketAddr, router: Arc<TcpRouter>)-> impl Future<Item=(), Error=Error> {
    let rt = router.clone();
    ParseFirstPacket::new(client_stream, a, rt)
        .and_then(move|tcp| {
            let r = router.route(a, tcp.protocol);
            let client_stream = tcp.stream;
            let p = client_stream.peer_addr().unwrap();
            if let Some(r) = r {
                Either::A(carry_out(tcp.bytes.freeze(), a, r, client_stream))
            } else {
                Either::B(future::err(format_err!("No matching rule for tcp from {:?} to {:?}", p, a)))
            }
    }).map_err(|e| e.into())
}

fn carry_out(data: Bytes, a: SocketAddr, r: Route, client_stream: TcpStream)-> impl Future<Item=(), Error=Error> {
    let s = match r {
        Route::Reset => return Either::A(future::ok(())),
        Route::Direct => tokio::net::TcpStream::connect(&a),
        Route::Socks(x) => tokio::net::TcpStream::connect(&a),
    }.map_err(|e| e.into());
    Either::B(s
        .and_then(|stream| write_all(stream, data).map_err(|e| e.into()))
        .and_then(|(stream, _)| {
            let (ur, uw) = stream.split();
            let (cr, cw) = client_stream.split();
            tokio::spawn(run_copy(ur, cw));
            tokio::spawn(run_copy(cr, uw));
            Ok(())
        })
    )
}

fn run_copy<R, W>(reader: R, writer: W) -> impl Future<Item=(), Error=()>
    where R: AsyncRead,
          W: AsyncWrite, {
    copy(reader, writer).map(|_x| ())
        .map_err(|e| error!("Error copying {:?}", e))
}

