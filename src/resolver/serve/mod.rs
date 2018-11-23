use failure::Error;
use std::io;

use tokio;
use std::sync::Arc;
use std::net::SocketAddr;

use crate::conf::DomainMatcher;
use crate::resolver::handler;
use crate::conf::DnsProxy;
use crate::resolver::handler::SmartResolver;
use tokio::reactor::Handle;
use std::net::UdpSocket as UdpSocketStd;
use tokio::net::UdpSocket;

pub fn serve(conf: DnsProxy, matcher: Arc<DomainMatcher>) -> Result<(), Error> {
    let handler = handler::SmartResolver::new(matcher, &conf)?;
    let handler = Arc::new(handler);
    let addr = conf.listen;

    let sock_std = UdpSocketStd::bind(addr)?;

    tokio::spawn_async(  async move{
        loop {
            let (sock, mut buf, size, peer) = match await!(recv_req(&sock_std)) {
                Ok(x) => x,
                Err(e) => {
                    warn!("Error receiving datagram: {}", e);
                    continue
                }
            };
            buf.truncate(size);
            let h = (&handler).clone();
            tokio::spawn_async(async move {
                if let Err(e) = await!(handle_dns_client(&buf, peer, sock, h)) {
                    error!("Error handling dns client: {:?}", e);
                }
            });
        }
    });
    Ok(())
}

async fn recv_req(sock: &UdpSocketStd)-> io::Result<(UdpSocket, Vec<u8>, usize, SocketAddr)> {
    let sock_tok = UdpSocket::from_std(sock.try_clone()?, &Handle::current())?;
    let buf = vec![0; 998];
    await!(sock_tok.recv_dgram(buf))
}

async fn handle_dns_client(data: &[u8], peer: SocketAddr, sock: UdpSocket, handler: Arc<SmartResolver>)-> Result<(), Error> {
    let x = await!(handler.handle_future( & data))?;
    await!(sock.send_dgram( & x, &peer))?;
    Ok(())
}