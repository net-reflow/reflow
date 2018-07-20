use std::net::SocketAddr;
use std::io;
use std::io::Read;
use std::fs;
use std::path;
use std::collections::BTreeMap;

use failure::Error;
use bytes::Bytes;

use toml;

#[derive(Debug)]
pub struct DnsProxyConf {
    pub listen: SocketAddr,
    pub resolv: BTreeMap<Bytes, DnsUpstream>,
    pub default: DnsUpstream,
}

/// Address of upstream dns server
/// with optionally a socks proxy
#[derive(Deserialize, Debug, Clone)]
pub struct DnsUpstream {
    pub addr: SocketAddr,
    pub socks5: Option<SocketAddr>,
}

impl DnsProxyConf {
    pub fn new(confpath: &path::Path) -> Result<DnsProxyConf, Error> {
        let contents = fs::read_to_string(confpath)?;

        #[derive(Deserialize, Debug)]
        struct ConfFileContent {
            listen: SocketAddr,
            server: BTreeMap<String, DnsUpstream>,
            rule: BTreeMap<String, String>,
        }

        let mut conf: ConfFileContent = toml::from_str(&contents)?;
        println!("cfc {:?}", conf);
        let servers = conf.server;
        let default = conf.rule.remove("else").and_then(|s| {
            servers.get(&s)
        }).ok_or(io::Error::new(io::ErrorKind::NotFound, "no default dns server defined"))?;
        let mut resolv =  BTreeMap::new();
        for (region, server) in conf.rule {
            let server_addr = servers.get(&server).ok_or(
             io::Error::new(io::ErrorKind::NotFound, format!("dns server {} not defined", server)))?;
            let up: DnsUpstream = server_addr.clone();
            let k = region.as_bytes().into();
            resolv.insert(k, up);
        }
        Ok(DnsProxyConf {
            listen: conf.listen,
            resolv: resolv,
            default: default.clone(),
        })
    }
}
