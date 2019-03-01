use tokio::net::TcpStream;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio_io::io::write_all;
use tokio;
use failure::Error;
use std::net::SocketAddr;
use net2::TcpBuilder;

mod copy;
use std::sync::Arc;
use crate::relay::TcpRouter;
use bytes::Bytes;
use tokio::reactor::Handle;
use self::copy::copy_verbose;
use std::net::IpAddr;
use std::net;
use crate::conf::RoutingAction;
use crate::conf::EgressAddr;
use crate::relay::inspect::parse_first_packet;
use crate::relay::inspect::TcpProtocol;
use asocks5::connect_socks_to;
use asocks5::socks::Address;

pub async fn handle_incoming_tcp(
    mut client_stream: TcpStream,
    a: SocketAddr,
    router: Arc<TcpRouter>,
)-> Result<(), Error> {
    let tcp = await!(parse_first_packet(&mut client_stream))?;
    if let Some(r) = router.route(a, &tcp.protocol) {
        await!(carry_out(
                tcp.bytes.freeze(), a, r.clone(), client_stream,
                tcp.protocol,
        ))?;
    } else {
        let p = client_stream.peer_addr();
        return Err(format_err!(
                "No matching rule for protocol {:?} from client {:?} to addr {:?}",
                &tcp.protocol, p, a
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
)-> Result<(), Error> {
    let s = match r {
        RoutingAction::Reset => return Ok(()),
        RoutingAction::Direct => {
            await!(tokio::net::TcpStream::connect(&a))
                .map_err(|e| format_err!("Error making direct {:?} connection to {:?}: {}", &pr, a, e))
        },
        RoutingAction::Named(ref g) => match g.val().addr() {
            EgressAddr::From(ip) => {
                let x = bind_tcp_socket(ip)?;
                await!(tokio::net::TcpStream::connect_std(x, &a, &Handle::default()))
                    .map_err(|e| format_err!("Error making direct {:?} connection to {:?} from {:?}: {}", &pr, a, ip, e))
            },
            EgressAddr::Socks5(x)=> {
                let mut s = await!(TcpStream::connect(&x))?;
                await!(connect_socks_to(&mut s, Address::SocketAddress(a)))?;
                Ok(s)
            },
        }
    }?;
    let (stream, _) = await!(write_all(s, data))
        .map_err(|e| {
            format_err!("Error sending {:?} header bytes to {:?}: {}", &pr, a, e)
        })?;
    let (ur, uw) = stream.split();
    let (cr, cw) = client_stream.split();
    run_copy(ur, cw, a, pr.clone(), r.clone(), true);
    run_copy(cr, uw, a, pr, r, false);
    Ok(())
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


fn bind_tcp_socket(ip: IpAddr)-> Result<net::TcpStream, Error> {
    let builder = if ip.is_ipv4() { TcpBuilder::new_v4() } else { TcpBuilder::new_v6() }?;
    let builder = builder.bind((ip, 0))?;
    builder.to_tcp_stream().map_err(|e| e.into())
}