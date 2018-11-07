use tokio::net::TcpStream;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio_io::io::write_all;
use tokio;
use futures::{Future, IntoFuture};
use futures::future;
use futures::future::Either;
use failure::Error;
use std::net::SocketAddr;
use futures_cpupool::CpuPool;
use net2::TcpBuilder;

mod copy;
use std::sync::Arc;
use crate::relay::TcpRouter;
use bytes::Bytes;
use tokio::io;
use socks::Socks5Stream;
use std::time::Duration;
use tokio::reactor::Handle;
use self::copy::copy_verbose;
use crate::util::Either3;
use std::net::IpAddr;
use std::net;
use crate::conf::RoutingAction;
use crate::conf::EgressAddr;
use crate::relay::inspect::ParseFirstPacket;
use crate::relay::inspect::TcpProtocol;

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
                    tcp.bytes.freeze(), a, r.clone(), client_stream, pool,
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
    r: RoutingAction,
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
        RoutingAction::Reset => return Either::A(future::ok(())),
        RoutingAction::Direct => Either3::A(
            tokio::net::TcpStream::connect(&a)
                .map_err(move|e| format_err!("Error making direct {:?} connection to {:?}: {}", p1, a, e))
        ),
        RoutingAction::Named(ref g) => match g.val().addr() {
            EgressAddr::From(ip) => Either3::B(
                bind_tcp_socket(ip)
                    .into_future()
                    .and_then(move |x| {
                        tokio::net::TcpStream::connect_std(x, &a, &Handle::current())
                    })
                    .map_err(move|e| format_err!("Error making direct {:?} connection to {:?} from {:?}: {}", p1, a, ip, e))
            ),
            EgressAddr::Socks5(x)=> Either3::C(
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
        }
    };
    Either::B(s
        .and_then(move |stream| write_all(stream, data)
            .map_err(move |e| format_err!("Error sending {:?} header bytes to {:?}: {}", p2, a, e)))
        .and_then(move |(stream, _)| {
            let (ur, uw) = stream.split();
            let (cr, cw) = client_stream.split();
            run_copy(ur, cw, a, p4, r.clone(), true);
            run_copy(cr, uw, a, p5, r, false);
            Ok(())
        })
    )
}

fn run_copy<R, W>(reader: R, writer: W, a: SocketAddr, p: TcpProtocol, r: RoutingAction, s_to_c: bool)
    where R: AsyncRead + Send + 'static,
          W: AsyncWrite + Send + 'static {
    tokio::spawn_async(async move {
        if let Err(e) = await!(copy_verbose(reader, writer)) {
            if s_to_c {
                if e.is_read() {
                    warn!("Error reading proto {:?} from server {:?} via {}: {}", p, a, r, e);
                }
            } else if !e.is_read() {
                warn!("Error writing proto {:?} to server {:?} via {}: {}", p, a, r, e);
            }
        }
    })
}


fn bind_tcp_socket(ip: IpAddr)-> io::Result<net::TcpStream> {
    let builder = if ip.is_ipv4() { TcpBuilder::new_v4() } else { TcpBuilder::new_v6() }?;
    let builder = builder.bind((ip, 0))?;
    builder.to_tcp_stream()
}