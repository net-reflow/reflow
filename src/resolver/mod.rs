/// smart dns resolver

use std::error::Error;
use std::time::Duration;
use std::sync::Arc;

use tokio;
use tokio::net;
use futures::Future;
use futures::future;

mod handler;
mod dnsclient;
mod monitor_failure;
mod server_future;
mod config;
mod util;

use super::ruling::DomainMatcher;
use self::config::DnsProxyConf;

pub fn start_resolver(router: Arc<DomainMatcher>, config: &str)-> Result<Box<Future<Item=(), Error=()> + Send>, Box<Error>>{
    let conf = DnsProxyConf::new(config.into())?;
    let handler = handler::SmartResolver::new(router, &conf)?;
    let server = server_future::ServerFuture::new(handler)?;
    let udpsock = net::UdpSocket::bind(&conf.listen)?;
    let uf = server.listen_udp(udpsock);
    let tcp = net::TcpListener::bind(&conf.listen)?;
    Ok(Box::new(future::lazy (move|| {
        server.listen_tcp(tcp, Duration::from_secs(5))
            .expect("fail to listen tcp");
        tokio::spawn(uf);
        Ok::<_, ()>(())
    })))
}
