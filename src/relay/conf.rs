use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::net::IpAddr;

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

