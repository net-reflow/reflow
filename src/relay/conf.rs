use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::net::IpAddr;
use std::path;
use failure::Error;
use std::fs;
use toml;
use bytes::Bytes;
use conf::EgressAddr;
use conf::Egress;
use trust_dns_resolver::name_server_pool::NameServer;
use conf::DnsProxy;

#[derive(Deserialize, Debug, Clone)]
pub struct AppConf {
    pub relay: RelayConf,
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
struct GatewayConf {
    kind: String,
    host: Option<IpAddr>,
    port: Option<u16>,
    ip: Option<IpAddr>,
}

impl GatewayConf {
    fn parse(&self)-> EgressAddr {
        match self.kind.as_ref() {
            "socks5" => EgressAddr::Socks5(SocketAddr::from((self.host.unwrap(), self.port.unwrap()))),
            "bind" => EgressAddr::From(self.ip.unwrap()),
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

    pub fn get_gateways(&self)-> BTreeMap<Bytes, Egress> {
        self.gateway.iter()
            .map(|(k,v)| -> (Bytes, Egress) {
                let k:&str = k.as_ref();
                let g = v.parse();
                let e = Egress {name: k.into(), addr: g};
                (k.into(), e)
            }).collect()
    }
}