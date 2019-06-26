use failure::Error;
use std::io;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio;

use crate::conf::DnsProxy;
use crate::conf::DomainMatcher;
use crate::resolver::handler;
use crate::resolver::handler::SmartResolver;
use futures::compat::Future01CompatExt;
use futures::task::Spawn;
use futures::task::SpawnExt;
use std::net::UdpSocket as UdpSocketStd;
use tokio::net::UdpSocket;
use tokio::reactor::Handle;

pub fn serve(
    conf: DnsProxy,
    matcher: Arc<DomainMatcher>,
    mut spawner: impl Spawn + Clone + Send + 'static,
) -> Result<(), Error> {
    let handler = handler::SmartResolver::new(matcher, &conf)?;
    let handler = Arc::new(handler);
    let addr = conf.listen;

    let sock_std = UdpSocketStd::bind(addr)?;

    let mut s1 = spawner.clone();
    if let Err(e) = spawner.spawn(async move {
        loop {
            let (sock, mut buf, size, peer) = match await!(recv_req(&sock_std)) {
                Ok(x) => x,
                Err(e) => {
                    warn!("Error receiving datagram: {}", e);
                    continue;
                }
            };
            buf.truncate(size);
            let h = (&handler).clone();
            if let Err(e) = s1.spawn(async move {
                if let Err(e) = await!(handle_dns_client(&buf, peer, sock, h)) {
                    error!("Error handling dns client: {:?}", e);
                }
            }) {
                error!("Error spawning: {:?}", e);
            }
        }
    }) {
        error!("Error spawning: {:?}", e);
    }
    Ok(())
}

async fn recv_req(sock: &UdpSocketStd) -> io::Result<(UdpSocket, Vec<u8>, usize, SocketAddr)> {
    let sock_tok = UdpSocket::from_std(sock.try_clone()?, &Handle::default())?;
    let buf = vec![0; 998];
    await!(sock_tok.recv_dgram(buf).compat())
}

async fn handle_dns_client(
    data: &[u8],
    peer: SocketAddr,
    sock: UdpSocket,
    handler: Arc<SmartResolver>,
) -> Result<(), Error> {
    let x = await!(handler.handle_future(&data))?;
    await!(sock.send_dgram(&x, &peer).compat())?;
    Ok(())
}
