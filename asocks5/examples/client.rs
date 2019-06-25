#![feature(await_macro, async_await)]

use asocks5::connect_socks_socket_addr;
use asocks5::socks::SocksError;
use futures::compat::Future01CompatExt;
use std::net::SocketAddr;
use structopt::StructOpt;
use tokio::net::TcpStream;
use tokio_io::io::{read, write_all};

fn main() {
    println!("Async socks client");
    let opt: Opt = Opt::from_args();
    let f = async move {
        if let Err(e) = await!(socks_example(opt.socks, opt.target, opt.data.as_bytes())) {
            eprintln!("Error running socks client: {:?}", e);
        }
    };
    futures::executor::block_on(f);
}

async fn socks_example(
    socks: SocketAddr,
    target: SocketAddr,
    data: &[u8],
) -> Result<(), SocksError> {
    println!("connecting to socks server");
    let mut s = await!(TcpStream::connect(&socks).compat())?;

    println!("connecting to remote server");
    await!(connect_socks_socket_addr(&mut s, target))?;

    println!("sending data");
    let (_s, _b) = await!(write_all(&s, data).compat())?;

    println!("reading data");
    let mut buf = [0u8; 2048];
    let (_s, buf, n) = await!(read(&s, &mut buf[..]).compat())?;
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
    #[structopt(parse(try_from_str), default_value = "1.1.1.1:80")]
    pub target: SocketAddr,
    #[structopt(short = "d", long = "data")]
    /// Remote server to access
    #[structopt(
        default_value = "GET / HTTP/1.1\r\nHost: 1.1.1.1\r\nUser-Agent: curl/7.52.1\r\nAccept: */*\r\n\r\n"
    )]
    pub data: String,
}
