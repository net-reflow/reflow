/// smart dns resolver

use std::net;
use std::str::FromStr;

use tokio_core::reactor::Handle;

mod handler;
mod server_future;

pub fn start_resolver(h: Handle){
    let sa = net::SocketAddr::new (net::IpAddr::from_str("8.8.8.8").unwrap(), 53);
    let handler = handler::SmartResolver::new(sa);
    let server = server_future::ServerFuture::new(handler).unwrap();
    let udpsock = net::UdpSocket::bind("127.0.0.1:7863").unwrap();
    server.listen_udp(udpsock,h.clone());
}
