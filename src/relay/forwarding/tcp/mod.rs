use failure::Error;
use futures::compat::Future01CompatExt;
use net2::TcpBuilder;
use std::net::SocketAddr;
use tokio;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::net::TcpStream;
use tokio_io::io::write_all;

mod copy;
use self::copy::copy_verbose;
use crate::conf::EgressAddr;
use crate::conf::RoutingAction;
use crate::relay::inspect::parse_first_packet;
use crate::relay::inspect::TcpProtocol;
use crate::relay::TcpRouter;
use bytes::Bytes;
use std::net;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::reactor::Handle;

use futures::task::{Spawn, SpawnExt};
use asocks5::connect_socks_socket_addr;

pub async fn handle_incoming_tcp(
    mut client_stream: TcpStream,
    a: SocketAddr,
    router: Arc<TcpRouter>,
    spawner: impl Spawn + Clone,
) -> Result<(), Error> {
    let tcp = await!(parse_first_packet(&mut client_stream))?;
    if let Some(r) = router.route(a, &tcp.protocol) {
        await!(carry_out(
            tcp.bytes.freeze(),
            a,
            r.clone(),
            client_stream,
            tcp.protocol,
            spawner,
        ))?;
    } else {
        let p = client_stream.peer_addr();
        return Err(format_err!(
            "No matching rule for protocol {:?} from client {:?} to addr {:?}",
            &tcp.protocol,
            p,
            a
        ));
    }
    Ok(())
}

async fn carry_out(
    data: Bytes,
    a: SocketAddr,
    r: RoutingAction,
    client_stream: TcpStream,
    pr: TcpProtocol,
    spawner: impl Spawn + Clone,
) -> Result<(), Error> {
    let s = match r {
        RoutingAction::Reset => return Ok(()),
        RoutingAction::Direct => await!(tokio::net::TcpStream::connect(&a).compat())
            .map_err(|e| format_err!("Error making direct {:?} connection to {:?}: {}", &pr, a, e)),
        RoutingAction::Named(ref g) => match g.val().addr() {
            EgressAddr::From(ip) => {
                let x = bind_tcp_socket(ip)?;
                await!(tokio::net::TcpStream::connect_std(x, &a, &Handle::default()).compat())
                    .map_err(|e| {
                        format_err!(
                            "Error making direct {:?} connection to {:?} from {:?}: {}",
                            &pr,
                            a,
                            ip,
                            e
                        )
                    })
            }
            EgressAddr::Socks5(x) => {
                let mut s = TcpStream::connect(&x).compat().await?;
                connect_socks_socket_addr(&mut s, a).await?;
                Ok(s)
            }
        },
    }?;
    let (stream, _) = await!(write_all(s, data).compat())
        .map_err(|e| format_err!("Error sending {:?} header bytes to {:?}: {}", &pr, a, e))?;
    let (ur, uw) = stream.split();
    let (cr, cw) = client_stream.split();
    run_copy(ur, cw, a, pr.clone(), r.clone(), true, spawner.clone());
    run_copy(cr, uw, a, pr, r, false, spawner);
    Ok(())
}

fn run_copy<R, W>(
    reader: R,
    writer: W,
    a: SocketAddr,
    p: TcpProtocol,
    r: RoutingAction,
    s_to_c: bool,
    mut spawn: impl Spawn,
) where
    R: AsyncRead + Send + 'static,
    W: AsyncWrite + Send + 'static,
{
    if let Err(e) = spawn.spawn(async move {
        if let Err(e) = await!(copy_verbose(reader, writer).compat()) {
            if s_to_c {
                if e.is_read() {
                    warn!(
                        "Error reading proto {:?} from server {:?} via {}: {}",
                        p, a, r, e
                    );
                }
            } else if !e.is_read() {
                warn!(
                    "Error writing proto {:?} to server {:?} via {}: {}",
                    p, a, r, e
                );
            }
        }
    }) {
        error!("spawning error {:?}", e);
    }
}

fn bind_tcp_socket(ip: IpAddr) -> Result<net::TcpStream, Error> {
    let builder = if ip.is_ipv4() {
        TcpBuilder::new_v4()
    } else {
        TcpBuilder::new_v6()
    }?;
    let builder = builder.bind((ip, 0))?;
    builder.to_tcp_stream().map_err(|e| e.into())
}
