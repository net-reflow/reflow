use std::net::SocketAddr;
use futures::Future;
use trust_dns_resolver::ResolverFuture;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::config::{NameServerConfig, Protocol};

pub fn create_resolver(addr: SocketAddr) -> ResolverFuture {
    let nc = NameServerConfig {
        socket_addr: addr,
        protocol: Protocol::Udp,
        tls_dns_name: None,
    };
    let conf = ResolverConfig::from_parts(None, vec![], vec![nc]);
    let r = ResolverFuture::new(conf, ResolverOpts::default());
    r.wait().expect("Can't get ResolverFuture")
}