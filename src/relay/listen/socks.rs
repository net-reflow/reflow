use failure::Error;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio;

use crate::relay::forwarding::handle_incoming_tcp;
use crate::relay::TcpRouter;
use asocks5::listen::handle_socks_handshake;
use asocks5::socks::Address;
use asocks5::socks::SocksError;
use asocks5::socks::TcpRequestHeader;
use asocks5::Command;
use tokio::net::TcpStream;

use futures::future::ready;

use crate::resolver::AsyncResolver;
use tokio::prelude::*;

pub async fn listen_socks(
    addr: &SocketAddr,
    resolver: Arc<AsyncResolver>,
    router: Arc<TcpRouter>,
) -> Result<(), Error> {
    let l = tokio::net::TcpListener::bind(addr).await?;
    let l = l.incoming();
    let f = l.for_each(move |s| {
        match s {
            Ok(s) => {
                let r1 = resolver.clone();
                let rt1 = router.clone();
                tokio::spawn(async move {
                    let _r = handle_client(s, r1, rt1).await.map_err(|e| {
                        error!("error handling client {}", e);
                    });
                });
            }
            Err(e) => error!("error incoming {:?}", e),
        }
        ready(())
    });
    tokio::spawn(f);
    Ok(())
}

async fn handle_client(
    s: TcpStream,
    res: Arc<AsyncResolver>,
    rt: Arc<TcpRouter>,
) -> Result<(), Error> {
    let (s, req) = handle_socks_handshake(s).await?;
    let a = read_address(req, res.clone()).await?;
    handle_incoming_tcp(s, a, rt).await
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
            let ips = resolver
                .resolve(&*domain)
                .await
                .map_err(|e| format_err!("Error resolving {}: {}", domain, e))?;
            let ip = ips
                .into_iter()
                .next()
                .ok_or_else(|| format_err!("No address found for domain {}", domain))?;
            Ok(SocketAddr::new(ip, port))
        }
    }
}
