use asocks5::listen::handle_socks_handshake;
use asocks5::socks::Address;
use asocks5::Command;
use failure::{format_err, Error};
use futures::compat::Future01CompatExt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use structopt::StructOpt;
use tokio;
use tokio::net::TcpStream;
use tokio::prelude::*;
use trust_dns_resolver::AsyncResolver;

use futures::executor::LocalPool;

use futures::task::SpawnExt;

// the main function is actually no different from any ordinary tcp server
fn main() -> Result<(), Error> {
    println!("Async socks server");
    let opt: Opt = Opt::from_args();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), opt.port);
    println!("listening on {:?}", addr);
    let mut executor = LocalPool::new();
    let mut spawner = executor.spawner();
    executor.run_until(async move {
        let listner = tokio::net::TcpListener::bind(&addr).await.unwrap();
        let _result = listner
            .incoming()
            .for_each(|s| {
                match s {
                    Ok(s) => {
                        let peer = s.peer_addr().expect("socket peer address");
                        println!("got tcp connection from {:?}", peer);
                        if let Err(e) = spawner.spawn(async move {
                            if let Err(e) = handle_client(s).await {
                                eprintln!("error handling client {}", e);
                            };
                        }) {
                            eprintln!("spawn error {:?}", e);
                        }
                    }
                    Err(e) => eprintln!("listen error {}", e),
                }
                futures::future::ready(())
            })
            .await;
    });
    Ok(())
}

async fn handle_client(client_stream: TcpStream) -> Result<(), Error> {
    println!("doing socks handshake");
    let (mut client_stream, req) = handle_socks_handshake(client_stream).await?;
    println!("socks client requests {:?}", req);
    if req.command != Command::TcpConnect {
        return Err(format_err!("this example only handles TcpConnect requests"));
    }

    // now there is the TcpStream and the socks TcpRequestHeader, you can do anything with it

    let addr = resolve_address(req.address).await?;
    println!("connecting to remote target {:?}", addr);
    let mut target_stream = TcpStream::connect(&addr).await?;

    // to create a useful socks server, you probably want to run loops
    // but this is just a simple example

    println!("reading payload from client");
    let mut buf = [0u8; 2048];
    let n = client_stream.read(&mut buf[..]).await?;
    println!("forwarding {} bytes to target server", n);
    target_stream.write_all(&buf[..n]).await?;
    println!("reading reply from target server");
    let n = target_stream.read(&mut buf[..]).await?;
    println!("forwarding reply to client");
    client_stream.write_all(&buf[..n]).await?;
    Ok(())
}

async fn resolve_address(address: Address) -> Result<SocketAddr, Error> {
    match address {
        Address::SocketAddress(a) => Ok(a),
        Address::DomainNameAddress(domain, port) => {
            println!("resolving remote address {}", domain);
            let (resolver, bg) = AsyncResolver::from_system_conf()?;
            tokio::spawn(async move {
                bg.compat().await.unwrap();
            });
            let lookup = resolver.lookup_ip(&*domain).compat().await?;
            let ip = lookup
                .iter()
                .next()
                .ok_or_else(|| format_err!("No address found for domain {}", domain))?;
            Ok(SocketAddr::new(ip, port))
        }
    }
}

#[derive(StructOpt)]
struct Opt {
    #[structopt(short = "p", long = "port")]
    /// Socks server port
    #[structopt(default_value = "1080")]
    pub port: u16,
}
