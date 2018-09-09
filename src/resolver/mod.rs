/// smart dns resolver

use std::net;
use std::time::Duration;
use std::sync::Arc;

use tokio_core::reactor::Handle;

mod handler;
mod dnsclient;
mod server_future;
mod config;

use super::ruling::Ruler;

pub fn start_resolver(h: Handle, router: Arc<Ruler>){
    let handler = handler::SmartResolver::new( h.clone(), router).unwrap();
    let server = server_future::ServerFuture::new(handler).unwrap();
    let listenaddr: net::SocketAddr = "127.0.0.1:7863".parse().unwrap();
    let udpsock = net::UdpSocket::bind(listenaddr).unwrap();
    server.listen_udp(udpsock,h.clone());
    let tcp = net::TcpListener::bind(listenaddr).unwrap();
    server.listen_tcp(tcp, Duration::from_secs(5), h.clone());
}
