use failure::Error;
use std::io;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio;

use crate::conf::DnsProxy;
use crate::conf::DomainMatcher;
use crate::resolver::handler;
use crate::resolver::handler::SmartResolver;

use std::net::UdpSocket as UdpSocketStd;
use tokio::net::UdpSocket;
use tokio_net::driver::Handle;

pub fn serve(conf: DnsProxy, matcher: Arc<DomainMatcher>) -> Result<(), Error> {
    let handler = handler::SmartResolver::new(matcher, &conf)?;
    let handler = Arc::new(handler);
    let addr = conf.listen;

    let sock_std = UdpSocketStd::bind(addr)?;

    tokio::spawn(async move {
        loop {
            let (sock, mut buf, size, peer) = match recv_req(&sock_std).await {
                Ok(x) => x,
                Err(e) => {
                    warn!("Error receiving datagram: {}", e);
                    continue;
                }
            };
            buf.truncate(size);
            let h = (&handler).clone();
            tokio::spawn(async move {
                if let Err(e) = handle_dns_client(&buf, peer, sock, h).await {
                    error!("Error handling dns client: {:?}", e);
                }
            });
        }
    });
    Ok(())
}

async fn recv_req(sock: &UdpSocketStd) -> io::Result<(UdpSocket, Vec<u8>, usize, SocketAddr)> {
    let mut sock_tok = UdpSocket::from_std(sock.try_clone()?)?;
    let mut buf = vec![0; 998];
    let (n, a) = sock_tok.recv_from(&mut buf).await?;
    Ok((sock_tok, buf, n, a))
}

async fn handle_dns_client(
    data: &[u8],
    peer: SocketAddr,
    mut sock: UdpSocket,
    handler: Arc<SmartResolver>,
) -> Result<(), Error> {
    let x = handler.handle_future(&data).await?;
    sock.send_to(&x, &peer).await?;
    Ok(())
}
