use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::net::IpAddr;
use std::path;
use failure::Error;
use std::fs;
use toml;
use bytes::Bytes;

#[derive(Deserialize, Debug, Clone)]
pub struct RelayConf {
    /// used to resolve
    pub resolver: SocketAddr,
    /// provide a socks5 server
    pub listen_socks: SocketAddr,
    gateway: BTreeMap<String, SocksConf>
}

#[derive(Clone, Deserialize, Debug)]
pub struct SocksConf {
    pub host: IpAddr,
    pub port: u16,
}

impl RelayConf {
    pub fn new(conf: &path::Path) -> Result<RelayConf, Error> {
        let p = conf.join("config.toml");
        let contents = fs::read_to_string(p)?;

        let conf: RelayConf = toml::from_str(&contents)?;
        Ok(conf)
    }

    pub fn get_gateways(&self)-> BTreeMap<Bytes, SocksConf> {
        self.gateway.iter()
            .map(|(k,v)| -> (Bytes, SocksConf) {
                let k:&str = k.as_ref();
                (k.into(), v.clone())
            }).collect()
    }
}