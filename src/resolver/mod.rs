/// smart dns resolver

use std::error::Error;
use std::time::Duration;
use std::sync::Arc;

use tokio;
use tokio::net;
use futures::Future;

mod handler;
mod dnsclient;
mod monitor_failure;
mod server_future;
mod config;

use super::ruling::DomainMatcher;
use self::config::DnsProxyConf;

pub fn start_resolver(router: Arc<DomainMatcher>, config: &str)-> Result<Box<Future<Item=(), Error=()>>, Box<Error>>{
    let conf = DnsProxyConf::new(config.into())?;
    let handler = handler::SmartResolver::new(router, &conf)?;
    let server = server_future::ServerFuture::new(handler)?;
    let udpsock = net::UdpSocket::bind(&conf.listen)?;
    server.listen_udp(udpsock);
    let tcp = net::TcpListener::bind(&conf.listen)?;
    let f = server.listen_tcp(tcp, Duration::from_secs(5));
    Ok(f)
}
