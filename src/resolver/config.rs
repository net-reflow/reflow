use std::net::SocketAddr;
use std::collections::BTreeMap;

use failure::Error;
use bytes::Bytes;

use relay::conf::DnsConf;
use relay::forwarding::Gateway;

#[derive(Debug)]
pub struct DnsProxyConf {
    pub listen: SocketAddr,
    pub resolv: BTreeMap<Bytes, DnsUpstream<Gateway>>,
    pub default: DnsUpstream<Gateway>,
}

/// Address of upstream dns server
/// with optionally a socks proxy
#[derive(Deserialize, Debug, Clone)]
pub struct DnsUpstream<T> {
    pub addr: SocketAddr,
    pub gateway: Option<T>,
}

/// replace named gateways with actual values
fn deref_route(d: DnsUpstream<String>, gw: &BTreeMap<Bytes, Gateway>)
               -> Result<DnsUpstream<Gateway>, Error> {
    let g = match d.gateway {
        Some(ref x) => {
            let x: &[u8] = x.as_bytes();
            Some(gw.get(x).ok_or_else(|| {
                format_err!("Missing gateway {:?} when configuring dns proxy", d.gateway)
            })?.clone())
        }
        None => None,
    };
    Ok(DnsUpstream {
        addr: d.addr,
        gateway: g,
    })
}

fn deref_server(name: &str, servers: &BTreeMap<String, DnsUpstream<String>>)
    ->Result<DnsUpstream<String>, Error> {
    servers.get(name)
        .map(|x| x.clone())
        .ok_or_else(|| format_err!("dns server {} not defined", name))
}

impl DnsProxyConf {
    pub fn from_conf(d: &DnsConf, gw: BTreeMap<Bytes, Gateway>) -> Result<DnsProxyConf, Error> {
        let servers = &d.server;
        let default = d.rule.get("else")
            .ok_or(format_err!("no default dns server defined"))
            .and_then(|s| deref_server(s, &servers))
            .and_then(|a| deref_route(a, &gw))?;
        let mut resolv =  BTreeMap::new();
        for (region, server) in &d.rule {
            let s = deref_server(&server, &servers)?;
            let up = deref_route(s, &gw)?;
            let k = region.as_bytes().into();
            resolv.insert(k, up);
        }
        Ok(DnsProxyConf {
            listen: d.listen,
            resolv: resolv,
            default: default.clone(),
        })
    }
}
