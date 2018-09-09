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
pub struct RegionalResolvers {
    pub resolv: Vec<(String, SocketAddr)>,
    pub default: SocketAddr,
}

#[derive(Deserialize, Debug)]
struct ConfValue {
    servers: BTreeMap<String, net::SocketAddr>,
    rule: Vec<BTreeMap<String, String>>,
}

impl RegionalResolvers{
    pub fn new(conf: path::PathBuf) -> Result<RegionalResolvers, Box<Error>> {
        let f = fs::File::open(conf).unwrap();
        let mut bufreader = io::BufReader::new(f);
        let mut contents = String::new();
        bufreader.read_to_string(&mut contents).unwrap();

        let conf: ConfValue = toml::from_str(&contents)?;
        let servers = conf.servers;
        let mut default: Option<SocketAddr> = None;
        let regions = conf.rule;
        let resolv = regions.iter().filter_map(|rs| {
            let region = rs.get("region").unwrap();
            let servername = rs.get("server").unwrap();
            let server = servers.get(servername).unwrap();
            if region == "else" {
                default = Some(server.clone());
                None
            } else {
                Some((region.clone(), server.clone()))
            }
        }).collect();
        let default = default.ok_or("no default dns server defined")?;
        Ok(RegionalResolvers{
            resolv: resolv,
            default: default,
        })
    }
}
