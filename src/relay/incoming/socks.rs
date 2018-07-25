use proto::socks::listen::listen;
use std::net::SocketAddr;
use std::sync::Arc;
use failure::Error;
use futures::{Stream, Future};
use tokio;
use trust_dns_resolver::ResolverFuture;

use relay::forwarding::handle_incoming_tcp;
use futures::future::Either;
use futures::future;
use tokio::net::TcpStream;
use proto::socks::Command;
use proto::socks::Address;
use proto::socks::SocksError;
use proto::socks::TcpRequestHeader;
use relay::TcpRouter;

pub fn listen_socks(addr: &SocketAddr, resolver: Arc<ResolverFuture>, router: Arc<TcpRouter>)->Result<(), Error >{
    let fut =
        listen(addr)?.for_each(move|s| {
            let r1 = resolver.clone();
            let rt1 = router.clone();
            let f =
                s.and_then(move|(s,h)| read_address(s, h, r1.clone()))
                .and_then(|(s,a)| handle_incoming_tcp(s, a, rt1))
                    .map_err(|e| error!("error handling client {}", e));
            tokio::spawn(f);
            Ok(())
        }).map_err(|e| error!("Listen error {:?}", e));
    tokio::spawn(fut);
    Ok(())
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

