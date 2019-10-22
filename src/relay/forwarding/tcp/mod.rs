use failure::Error;

use net2::TcpBuilder;
use std::net::SocketAddr;
use tokio;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::net::TcpStream;

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

use asocks5::connect_socks_socket_addr;

use tokio::prelude::*;
use tokio_io::split::split;
use tokio_net::driver::Handle;

pub async fn handle_incoming_tcp(
    mut client_stream: TcpStream,
    a: SocketAddr,
    router: Arc<TcpRouter>,
) -> Result<(), Error> {
    let tcp = parse_first_packet(&mut client_stream).await?;
    if let Some(r) = router.route(a, &tcp.protocol) {
        carry_out(
            tcp.bytes.freeze(),
            a,
            r.clone(),
            client_stream,
            tcp.protocol,
        )
        .await?;
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
) -> Result<(), Error> {
    let mut s = match r {
        RoutingAction::Reset => return Ok(()),
        RoutingAction::Direct => tokio::net::TcpStream::connect(&a)
            .await
            .map_err(|e| format_err!("Error making direct {:?} connection to {:?}: {}", &pr, a, e)),
        RoutingAction::Named(ref g) => match g.val().addr() {
            EgressAddr::From(ip) => {
                let x = bind_tcp_socket(ip)?;
                tokio::net::TcpStream::connect_std(x, &a, &Handle::default())
                    .await
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
                let mut s = TcpStream::connect(&x).await?;
                connect_socks_socket_addr(&mut s, a).await?;
                Ok(s)
            }
        },
    }?;
    s.write_all(data.as_ref())
        .await
        .map_err(|e| format_err!("Error sending {:?} header bytes to {:?}: {}", &pr, a, e))?;
    let (ur, uw) = split(s);
    let (cr, cw) = split(client_stream);
    run_copy(ur, cw, a, pr.clone(), r.clone(), true);
    run_copy(cr, uw, a, pr, r, false);
    Ok(())
}

fn run_copy<R, W>(
    reader: R,
    writer: W,
    a: SocketAddr,
    p: TcpProtocol,
    r: RoutingAction,
    s_to_c: bool,
) where
    R: AsyncRead + Send + 'static + Unpin,
    W: AsyncWrite + Send + 'static + Unpin,
{
    tokio::spawn(async move {
        if let Err(e) = copy_verbose(reader, writer).await {
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
    });
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
