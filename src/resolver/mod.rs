//! Proxy dns requests based on rules

use std::time::Duration;
use std::sync::Arc;

use tokio::net;
use failure::Error;

mod handler;
mod dnsclient;
mod monitor_failure;
mod config;
mod serve;
mod util;

use self::serve::ServerFuture;
use super::ruling::DomainMatcher;
use self::config::DnsProxyConf;
use std::net::SocketAddr;
use std::io;

/// Serve dns requests by forwarding
pub struct DnsServer {
    server: ServerFuture,
    /// Address and port it listens on
    addr: SocketAddr,
}

impl DnsServer {
    pub fn new(router: Arc<DomainMatcher>, config: &str) -> Result<DnsServer, Error> {
        let conf = DnsProxyConf::new(config.into())?;
        let handler = handler::SmartResolver::new(router, &conf)?;
        let server = ServerFuture::new(handler)?;
        let addr = conf.listen;
        Ok(DnsServer {
            server: server,
            addr: addr,
        })
    }

    pub fn start(&self)-> io::Result<()> {
        let udpsock = net::UdpSocket::bind(&self.addr)?;
        self.server.listen_udp(udpsock);
        let tcp = net::TcpListener::bind(&self.addr)?;
        self.server.listen_tcp(tcp, Duration::from_secs(5))?;
        Ok(())
    }
}