use failure::Error;
use futures::compat::Future01CompatExt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio;
use trust_dns_resolver::AsyncResolver;

use crate::relay::forwarding::handle_incoming_tcp;
use crate::relay::TcpRouter;
use asocks5::listen::handle_socks_handshake;
use asocks5::socks::Address;
use asocks5::socks::SocksError;
use asocks5::socks::TcpRequestHeader;
use asocks5::Command;
use futures01::stream::Stream;
use tokio::net::TcpStream;

use futures::executor::ThreadPool;
use futures::future::ready;
use futures::task::Spawn;
use futures::task::SpawnExt;
use futures::FutureExt;
use futures::TryFutureExt;

pub fn listen_socks(
    addr: &SocketAddr,
    resolver: Arc<AsyncResolver>,
    router: Arc<TcpRouter>,
    mut pool: ThreadPool,
) -> Result<(), Error> {
    let l = tokio::net::TcpListener::bind(addr)?;
    let l = l.incoming();
    let mut p2 = pool.clone();
    let f = l.for_each(move |s| {
        let r1 = resolver.clone();
        let rt1 = router.clone();
        let p1 = pool.clone();
        if let Err(e) = pool.spawn(async move {
            let _r = await!(handle_client(s, r1, rt1, p1)).map_err(|e| {
                error!("error handling client {}", e);
            });
        }) {
            error!("error spawning {:?}", e);
        }
        Ok(())
    });
    if let Err(e) = p2.spawn(
        f.compat()
            .map_err(|e| {
                error!("foreach error {}", e);
            })
            .then(|_| ready(())),
    ) {
        error!("spawn error {:?}", e);
    }
    Ok(())
}

async fn handle_client(
    s: TcpStream,
    res: Arc<AsyncResolver>,
    rt: Arc<TcpRouter>,
    spawner: impl Spawn + Clone,
) -> Result<(), Error> {
    let (s, req) = await!(handle_socks_handshake(s))?;
    let a = await!(read_address(req, res.clone()))?;
    await!(handle_incoming_tcp(s, a, rt, spawner))
}

async fn read_address(
    head: TcpRequestHeader,
    resolver: Arc<AsyncResolver>,
) -> Result<SocketAddr, Error> {
    let c = head.command;
    if c != Command::TcpConnect {
        return Err(SocksError::CommandUnSupport { cmd: c as u8 }.into());
    }
    match head.address {
        Address::SocketAddress(a) => Ok(a),
        Address::DomainNameAddress(domain, port) => {
            let lookup = await!(resolver.lookup_ip(&*domain).compat())
                .map_err(|e| format_err!("Error resolving {}: {}", domain, e))?;
            let ip = lookup
                .iter()
                .next()
                .ok_or_else(|| format_err!("No address found for domain {}", domain))?;
            Ok(SocketAddr::new(ip, port))
        }
    }
}
