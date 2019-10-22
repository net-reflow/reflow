use super::dnsclient::DnsClient;
use crate::conf::NameServer;

use failure::Error;
use std::net::IpAddr;
use std::str::FromStr;
use trust_dns::op::Message;
use trust_dns::op::Query;
use trust_dns::proto::rr::Name;
use trust_dns::rr::RecordType;
use trust_dns::serialize::binary::{BinDecodable, BinDecoder};

use failure::_core::time::Duration;
use tokio::future::FutureExt;

pub struct AsyncResolver {
    client: DnsClient,
}
impl AsyncResolver {
    pub fn new(rm: &NameServer) -> AsyncResolver {
        let client = DnsClient::new(rm);
        AsyncResolver { client }
    }

    pub async fn resolve(&self, name: &str) -> Result<Vec<IpAddr>, Error> {
        let result = self
            .resolve_eternal(name)
            .timeout(Duration::from_secs(10))
            .await?;
        result
    }
    async fn resolve_eternal(&self, name: &str) -> Result<Vec<IpAddr>, Error> {
        let mut msg = Message::new();
        let name = Name::from_str(name).unwrap();
        let query = Query::query(name, RecordType::A);
        msg.add_query(query);
        let res = self.client.resolve(msg.to_vec()?).await?;
        let mut decoder = BinDecoder::new(&res);
        let message = Message::read(&mut decoder).expect("msg deco err");
        let ips = message
            .answers()
            .into_iter()
            .filter_map(|rec| {
                let rd = rec.rdata();
                let ip = rd.to_ip_addr();
                ip
            })
            .collect();
        Ok(ips)
    }
}

#[cfg(test)]
mod tests {
    use crate::conf::{main::RefVal, Egress, EgressAddr};
    use crate::conf::{NameServer, NameServerRemote};
    use crate::resolver::AsyncResolver;
    use bytes::Bytes;
    use std::net::IpAddr;
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[test]
    fn udp_test() {
        let ip = [127, 8, 8, 8];
        let remote = NameServerRemote::Udp(SocketAddr::new(IpAddr::from(ip), 53001));
        let ns = NameServer {
            remote,
            egress: None,
        };
        let resolver = AsyncResolver::new(&ns);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let response = rt.block_on(async move { resolver.resolve("www.example.com.").await });
        assert!(response.is_err());
    }

    #[test]
    fn socks_test() {
        let ip = [8, 8, 8, 8];
        let remote = NameServerRemote::Tcp(SocketAddr::new(IpAddr::from(ip), 53));
        let ns = NameServer {
            remote,
            egress: Some(RefVal::Val(Egress {
                name: Bytes::new(),
                addr: EgressAddr::Socks5(SocketAddr::from_str("1.1.1.1:3128").unwrap()),
            })),
        };
        let resolver = AsyncResolver::new(&ns);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let response = rt.block_on(async move { resolver.resolve("www.example.com").await });
        assert!(response.is_err());
    }
}
