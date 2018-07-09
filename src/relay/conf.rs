use std::net::SocketAddr;
use std::path;
use failure::Error;
use std::fs;
use toml;

#[derive(Deserialize, Debug, Clone)]
pub struct RelayConf {
    /// used to resolve
    pub resolver: SocketAddr,
    /// provide a socks5 server
    pub listen_socks: SocketAddr,
}

impl RelayConf {
    pub fn new(conf: &path::Path) -> Result<RelayConf, Error> {
        let p = conf.join("config.toml");
        let contents = fs::read_to_string(p)?;

        let conf: RelayConf = toml::from_str(&contents)?;
        Ok(conf)
    }
}
