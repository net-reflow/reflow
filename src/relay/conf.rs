use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::net::IpAddr;
use std::path;
use failure::Error;
use std::fs;
use toml;
use bytes::Bytes;
use resolver::config::DnsUpstream;
use conf::Gateway;

#[derive(Deserialize, Debug, Clone)]
pub struct AppConf {
    pub relay: RelayConf,
    pub dns: DnsConf,
    gateway: BTreeMap<String, GatewayConf>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct RelayConf {
    /// use this dns server to get an ip when
    /// a socks client requests to connect to a domain
    pub resolver: SocketAddr,
    /// provide a socks5 server
    pub listen_socks: SocketAddr,
}

#[derive(Clone, Deserialize, Debug)]
pub struct DnsConf {
    /// start a smart dns server
    pub listen: SocketAddr,
    pub server: BTreeMap<String, DnsUpstream<String>>,
    pub rule: BTreeMap<String, String>,
}

#[derive(Clone, Deserialize, Debug)]
struct GatewayConf {
    kind: String,
    host: Option<IpAddr>,
    port: Option<u16>,
    ip: Option<IpAddr>,
}

impl GatewayConf {
    fn parse(&self)->Gateway {
        match self.kind.as_ref() {
            "socks5" => Gateway::Socks5(SocketAddr::from((self.host.unwrap(), self.port.unwrap()))),
            "bind" => Gateway::From(self.ip.unwrap()),
            x => panic!("Unknown gateway type {}", x),
        }
    }
}

impl AppConf {
    pub fn new(conf: &path::Path) -> Result<AppConf, Error> {
        let p = conf.join("config.toml");
        let contents = fs::read_to_string(p)?;

        let conf: AppConf = toml::from_str(&contents)?;
        Ok(conf)
    }

    pub fn get_gateways(&self)-> BTreeMap<Bytes, Gateway> {
        self.gateway.iter()
            .map(|(k,v)| -> (Bytes, Gateway) {
                let k:&str = k.as_ref();
                let g = v.parse();
                (k.into(), g)
            }).collect()
    }
}