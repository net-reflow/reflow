use std::net::SocketAddr;
use std::sync::Arc;
use failure::Error;
use tokio;
use tokio::prelude::*;
use trust_dns_resolver::ResolverFuture;

use crate::relay::forwarding::handle_incoming_tcp;
use tokio::net::TcpStream;
use crate::proto::socks::Command;
use crate::proto::socks::Address;
use crate::proto::socks::SocksError;
use crate::proto::socks::TcpRequestHeader;
use crate::relay::TcpRouter;
use crate::proto::socks::listen::handle_socks_handshake;

pub fn listen_socks(addr: &SocketAddr, resolver: Arc<ResolverFuture>, router: Arc<TcpRouter>)->Result<(), Error >{
    let l = tokio::net::TcpListener::bind(addr)?;
    let mut l = l.incoming();
    tokio::spawn_async(async move{
        while let Some(s) = await!(l.next()) {
            match s {
                Ok(s) => {
                    let r1 = resolver.clone();
                    let rt1 = router.clone();
                    tokio::spawn_async(async move {
                        let _r = await!(handle_client(s, r1, rt1)).map_err(|e| {
                            error!("error handling client {}", e);
                        });
                    });
                }
                Err(e) => {
                    error!("error getting next socks client {}", e);
                }
            }

        }
    });
    Ok(())
}

async fn handle_client(
    s: TcpStream,
    res: Arc<ResolverFuture>,
    rt: Arc<TcpRouter>,
) -> Result<(), Error>
{
    let (s, req) = await!(handle_socks_handshake(s))?;
    let a = await!(read_address(req, res.clone()))?;
    await!(handle_incoming_tcp(s, a, rt))
}

async fn read_address(head: TcpRequestHeader, resolver: Arc<ResolverFuture>)
    -> Result<SocketAddr, Error> {
    let c = head.command;
    if c != Command::TcpConnect {
        return Err(SocksError::CommandUnSupport { cmd: c as u8}.into());
    }
    match head.address {
        Address::SocketAddress(a) => return Ok(a),
        Address::DomainNameAddress(domain, port) => {
            let lookup = await!(resolver.lookup_ip(&*domain)).map_err(|e| {
                format_err!("Error resolving {}: {}", domain, e)
            })?;
            let ip = lookup.iter().next()
                .ok_or_else(||format_err!("No address found for domain {}", domain))?;
            Ok(SocketAddr::new(ip, port))
        }
    }
}

