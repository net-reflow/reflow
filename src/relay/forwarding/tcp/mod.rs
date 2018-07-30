use tokio::net::TcpStream;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio_io::io::write_all;
use tokio;
use futures::Future;
use futures::future;
use futures::future::Either;
use failure::Error;
use std::net::SocketAddr;
use futures_cpupool::CpuPool;

mod copy;
mod inspect;
use self::inspect::ParseFirstPacket;
pub use self::inspect::TcpProtocol;
use std::sync::Arc;
use relay::TcpRouter;
use bytes::Bytes;
use relay::forwarding::routing::Route;
use tokio::io;
use socks::Socks5Stream;
use std::time::Duration;
use tokio::reactor::Handle;
use self::copy::copy_verbose;

pub const TIMEOUT: u64 = 10;

pub fn handle_incoming_tcp(
    client_stream: TcpStream,
    a: SocketAddr,
    router: Arc<TcpRouter>,
    pool: CpuPool,
)-> impl Future<Item=(), Error=Error> {
    ParseFirstPacket::new(client_stream)
        .and_then(move|tcp| {
            let r = router.route(a, &tcp.protocol);
            let client_stream = tcp.stream;
            let p = client_stream.peer_addr();
            if let Some(r) = r {
                Either::A(carry_out(
                    tcp.bytes.freeze(), a, r, client_stream, pool,
                    tcp.protocol,
                ))
            } else {
                Either::B(future::err(format_err!(
                "No matching rule for protocol {:?} from client {:?} to addr {:?}",
                &tcp.protocol, p, a
                )))
            }
    })
}

fn carry_out(
    data: Bytes,
    a: SocketAddr,
    r: Route,
    client_stream: TcpStream,
    p: CpuPool,
    pr: TcpProtocol,
)-> impl Future<Item=(), Error=Error> {
    let p1 = pr.clone();
    let p2 = pr.clone();
    let p3 = pr.clone();
    let p4= pr.clone();
    let p5 = pr;
    let s = match r {
        Route::Reset => return Either::A(future::ok(())),
        Route::Direct => Either::A(
            tokio::net::TcpStream::connect(&a)
                .map_err(move|e| format_err!("Error making {:?} connection to {:?}: {}", p1, a, e))
        ),
        Route::Socks(x) => Either::B(
            p.spawn_fn(move || -> io::Result<Socks5Stream> {
                let ss = Socks5Stream::connect(x, a)?;
                ss.get_ref().set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;
                Ok(ss)
            }).and_then(|ss| {
                let ts = ss.into_inner();
                TcpStream::from_std(ts, &Handle::current())
            }).map_err(move |e| {
                format_err!("Error making {:?} connection to {:?} through {:?}: {}", p3, a, x, e)
            })
        ),
    };
    Either::B(s
        .and_then(move |stream| write_all(stream, data)
            .map_err(move |e| format_err!("Error sending {:?} header bytes to {:?}: {}", p2, a, e)))
        .and_then(move |(stream, _)| {
            let (ur, uw) = stream.split();
            let (cr, cw) = client_stream.split();
            tokio::spawn(run_copy(ur, cw, a, p4, r, "server to client"));
            tokio::spawn(run_copy(cr, uw, a, p5, r, "client to server"));
            Ok(())
        })
    )
}

fn run_copy<R, W>(reader: R, writer: W, a: SocketAddr, p: TcpProtocol, r: Route, d: &'static str) -> impl Future<Item=(), Error=()>
    where R: AsyncRead,
          W: AsyncWrite, {
    copy_verbose(reader, writer, d).map(|_x| ())
        .map_err(move |e| warn!("Error with {:?} with remote {:?} via {:?}: {}", p, a, r, e))
}
