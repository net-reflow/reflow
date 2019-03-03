#![feature(await_macro, async_await)]
#![feature(futures_api)]

use tokio::await;

use std::net::{SocketAddr, Ipv4Addr, IpAddr};
use failure::{Error, format_err};
use tokio;
use tokio::prelude::*;
use tokio_io::io::{write_all, read};
use tokio::net::TcpStream;
use asocks5::socks::Address;
use asocks5::listen::handle_socks_handshake;
use asocks5::Command;
use structopt::StructOpt;
use trust_dns_resolver::AsyncResolver;

// the main function is actually no different from any ordinary tcp server
fn main()->Result<(), Error >{
    println!("Async socks server");
    let opt: Opt = Opt::from_args();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), opt.port);
    println!("listening on {:?}", addr);
    let listner = tokio::net::TcpListener::bind(&addr)?;
    tokio::run_async(async move{
        let result = await!(listner.incoming().for_each(|s| {
            let peer = s.peer_addr().expect("socket peer address");
            println!("got tcp connection from {:?}", peer);
            tokio::spawn_async(async move {
                if let Err(e) = await!(handle_client(s)) {
                    eprintln!("error handling client {}", e);
                };
            });
            Ok(())
        }));
        if let Err(e) = result {
            eprintln!("error running TcpListener {:?}", e);
        }
    });
    Ok(())
}

async fn handle_client(client_stream: TcpStream) -> Result<(), Error> {
    println!("doing socks handshake");
    let (client_stream, req) = await!(handle_socks_handshake(client_stream))?;
    println!("socks client requests {:?}", req);
    if req.command != Command::TcpConnect {
        return Err(format_err!("this example only handles TcpConnect requests"));
    }

    // now there is the TcpStream and the socks TcpRequestHeader, you can do anything with it

    let addr = await!(resolve_address(req.address))?;
    println!("connecting to remote target {:?}", addr);
    let target_stream = await!(TcpStream::connect(&addr))?;

    // to create a useful socks server, you probably want to run loops
    // but this is just a simple example

    println!("reading payload from client");
    let mut buf = [0u8; 2048];
    let (_s, buf, n) = await!(read(&client_stream, &mut buf[..]))?;
    println!("forwarding {} bytes to target server", n);
    await!(write_all(&target_stream, &buf[..n]))?;
    println!("reading reply from target server");
    let (_s, buf, n) = await!(read(&target_stream, &mut buf[..]))?;
    println!("forwarding reply to client");
    await!(write_all(client_stream, &buf[..n]))?;
    Ok(())
}

async fn resolve_address(address: Address)-> Result<SocketAddr, Error> {
    match address {
        Address::SocketAddress(a) => return Ok(a),
        Address::DomainNameAddress(domain, port) => {
            println!("resolving remote address {}", domain);
            let (resolver, bg) = AsyncResolver::from_system_conf()?;
            tokio::spawn(bg);
            let lookup = await!(resolver.lookup_ip(&*domain))?;
            let ip = lookup.iter().next()
                .ok_or_else(||format_err!("No address found for domain {}", domain))?;
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