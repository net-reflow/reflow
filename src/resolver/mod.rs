/// smart dns resolver

use std::net;
use std::str::FromStr;
use std::time::Duration;

use tokio_core::reactor::Handle;

mod handler;
mod dnsclient;
mod server_future;

pub fn start_resolver(h: Handle){
    let sa = net::SocketAddr::new (net::IpAddr::from_str("8.8.8.8").unwrap(), 53);
    let handler = handler::SmartResolver::new(sa, h.clone()).unwrap();
    let server = server_future::ServerFuture::new(handler).unwrap();
    let listenaddr: net::SocketAddr = "127.0.0.1:7863".parse().unwrap();
    let udpsock = net::UdpSocket::bind(listenaddr).unwrap();
    server.listen_udp(udpsock,h.clone());
    let tcp = net::TcpListener::bind(listenaddr).unwrap();
    server.listen_tcp(tcp, Duration::from_secs(5), h.clone());
}
