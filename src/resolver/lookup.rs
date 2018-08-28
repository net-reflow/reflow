use futures::Future;
use trust_dns_resolver::ResolverFuture;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::config::{NameServerConfig, Protocol};
use conf::NameServerRemote;

pub fn create_resolver(rm: NameServerRemote) -> ResolverFuture {
    let (addr, p) = match rm {
        NameServerRemote::Udp(a) => (a, Protocol::Udp),
        NameServerRemote::Tcp(a) => (a, Protocol::Tcp),
    };
    let nc = NameServerConfig {
        socket_addr: addr,
        protocol: p,
        tls_dns_name: None,
    };
    let conf = ResolverConfig::from_parts(None, vec![], vec![nc]);
    let r = ResolverFuture::new(conf, ResolverOpts::default());
    r.wait().expect("Can't get ResolverFuture")
}