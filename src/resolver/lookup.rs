use std::io;
use std::net::SocketAddr;
use tokio::runtime::current_thread::Runtime;
use trust_dns_resolver::ResolverFuture;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::config::{NameServerConfig, Protocol};

fn new_resolver(addr: SocketAddr) -> Result<ResolverFuture, io::Error> {
    let nc = NameServerConfig {
        socket_addr: addr,
        protocol: Protocol::Udp,
        tls_dns_name: None,
    };
    let conf = ResolverConfig::from_parts(None, vec![], vec![nc]);
    let r = ResolverFuture::new(conf, ResolverOpts::default());
    let mut rt = Runtime::new().unwrap();
    let r = rt.block_on(r).expect("rf");
    Ok(r)
}