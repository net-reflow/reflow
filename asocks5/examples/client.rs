#![feature(await_macro, async_await)]
#![feature(futures_api)]

use tokio::await;
use tokio::net::TcpStream;
use tokio_io::io::{write_all, read};
use asocks5::connect_socks_to;
use asocks5::socks::Address;
use std::net::SocketAddr;
use asocks5::socks::SocksError;
use structopt::StructOpt;

fn main() {
    println!("Async socks client");
    let opt: Opt = Opt::from_args();
    tokio::run_async(async move {
        if let Err(e) = await!(socks_example(opt.socks, opt.target, opt.data.as_bytes())) {
            eprintln!("Error running socks client: {:?}", e);
        }
    });
}

async fn socks_example(socks: SocketAddr, target: SocketAddr, data: &[u8])-> Result<(), SocksError> {
    println!("connecting to socks server");
    let mut s = await!(TcpStream::connect(&socks))?;

    println!("connecting to remote server");
    await!(connect_socks_to(&mut s, Address::SocketAddress(target)))?;

    println!("sending data");
    let (_s, _b) = await!(write_all(&s, data))?;

    println!("reading data");
    let mut buf = [0u8; 2048];
    let (_s, buf, n) = await!(read(&s, &mut buf[..]))?;
    let rep = String::from_utf8_lossy(&buf[..n]);
    println!("read data: {}", rep);

    Ok(())
}

#[derive(StructOpt)]
struct Opt {
    #[structopt(short = "s", long = "socks")]
    /// Socks proxy server
    #[structopt(parse(try_from_str), default_value = "127.0.0.1:1080")]
    pub socks: SocketAddr,
    #[structopt(short = "t", long = "target")]
    /// Remote server to access
    #[structopt(parse(try_from_str), default_value ="1.1.1.1:80")]
    pub target: SocketAddr,
    #[structopt(short = "d", long = "data")]
    /// Remote server to access
    #[structopt(default_value ="GET / HTTP/1.1\r\nHost: 1.1.1.1\r\nUser-Agent: curl/7.52.1\r\nAccept: */*\r\n\r\n")]
    pub data: String,
}
