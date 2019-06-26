use crate::conf::NameServerRemote;
use futures::executor::ThreadPool;
use futures::task::SpawnExt;
use futures::compat::Future01CompatExt;
use trust_dns_resolver::config::{NameServerConfig, Protocol};
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::AsyncResolver;

pub fn create_resolver(rm: NameServerRemote,
                       mut pool: ThreadPool
) -> AsyncResolver {
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
    let (resolver, fut) = AsyncResolver::new(conf, ResolverOpts::default());
    pool.spawn(async move { fut.compat().await.expect("AsyncResolver"); }).expect("spawn AsyncResolver");
    resolver
}
