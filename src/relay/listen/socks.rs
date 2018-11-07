use crate::proto::socks::listen::listen;
use std::net::SocketAddr;
use std::sync::Arc;
use failure::Error;
use futures::{Stream, Future};
use tokio;
use trust_dns_resolver::ResolverFuture;

use crate::relay::forwarding::handle_incoming_tcp;
use futures::future::Either;
use futures::future;
use tokio::net::TcpStream;
use crate::proto::socks::Command;
use crate::proto::socks::Address;
use crate::proto::socks::SocksError;
use crate::proto::socks::TcpRequestHeader;
use crate::relay::TcpRouter;
use futures_cpupool::CpuPool;

pub fn listen_socks(addr: &SocketAddr, resolver: Arc<ResolverFuture>, router: Arc<TcpRouter>, p: CpuPool)->Result<(), Error >{
    let l = listen(addr).map_err(|e| format_err!("Fail to listen on socks {:?}: {}", addr, e))?;
    let fut =l.for_each(move|s| {
        let r1 = resolver.clone();
        let rt1 = router.clone();
        let p = p.clone();
        tokio::spawn_async(async move {
            let _r = await!(handle_client(s, r1, rt1, p)).map_err(|e| {
                error!("error handling client {}", e);
            });
        });
        Ok(())
    }).map_err(|e| error!("Listen error {:?}", e));
    tokio::spawn(fut);
    Ok(())
}

async fn handle_client<F>(
    s: F,
    res: Arc<ResolverFuture>,
    rt: Arc<TcpRouter>,
    p: CpuPool) -> Result<(), Error>
    where F: Future<Item=(TcpStream, TcpRequestHeader), Error=Error>,
{
    let (s, h) = await!(s)?;
    let (s, a) = await!(read_address(s, h, res.clone()))?;
    await!(handle_incoming_tcp(s, a, rt, p))
}

fn read_address(stream:TcpStream, head: TcpRequestHeader, resolver: Arc<ResolverFuture>)-> impl Future<Item=(TcpStream, SocketAddr), Error=Error> {
    let c = head.command;
    if c != Command::TcpConnect {
        return Either::A(future::err(SocksError::CommandUnSupport { cmd: c as u8}.into()));
    }
    match head.address {
        Address::SocketAddress(a) => return Either::A(future::ok((stream, a))),
        Address::DomainNameAddress(domain, port) => {
            Either::B(resolver.lookup_ip(&*domain)
                .then(move |res| {
                    match res {
                        Ok(lookup) => lookup.iter().next().ok_or_else(||format_err!("No address found for domain {}", domain)),
                        Err(e) => Err(format_err!("Error resolving {}: {}", domain, e)),
                    }
                })
                .map(move |ip| {
                (stream, SocketAddr::new(ip, port))
            }))
        }
    }
}

