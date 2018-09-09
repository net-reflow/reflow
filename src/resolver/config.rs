use std::net::SocketAddr;
use std::io;
use std::error::Error;
use std::io::Read;
use std::fs;
use std::net;
use std::path;
use std::collections::BTreeMap;

use toml;

#[derive(Debug)]
pub struct DnsProxyConf {
    pub listen: SocketAddr,
    pub resolv: BTreeMap<String, SocketAddr>,
    pub default: SocketAddr,
}

#[derive(Deserialize, Debug)]
struct ConfValue {
    listen: SocketAddr,
    servers: BTreeMap<String, net::SocketAddr>,
    rule: BTreeMap<String, String>,
}

impl DnsProxyConf {
    pub fn new(conf: path::PathBuf) -> Result<DnsProxyConf, Box<Error>> {
        let p = conf.join("resolve.config");
        let f = fs::File::open(p)?;
        let mut bufreader = io::BufReader::new(f);
        let mut contents = String::new();
        bufreader.read_to_string(&mut contents).unwrap();

        let conf: ConfValue = toml::from_str(&contents)?;
        let servers = conf.servers;
        let default = conf.rule.get("else").and_then(|s| {
            servers.get(s)
        }).ok_or(io::Error::new(io::ErrorKind::NotFound, "no default dns server defined"))?;
        let mut resolv =  BTreeMap::new();
        for (region, server) in &conf.rule {
            let server_addr = servers.get(server).ok_or(
             io::Error::new(io::ErrorKind::NotFound, format!("dns server {} defined", server)))?;
            resolv.insert(region.clone(), server_addr.clone());
        }
        Ok(DnsProxyConf {
            listen: conf.listen,
            resolv: resolv,
            default: default.clone(),
        })
    }
}
