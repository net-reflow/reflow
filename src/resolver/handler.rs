use std::sync::Arc;

use bytes::Bytes;
use failure::Error;
use futures::Future;
use futures::future;

use trust_dns::op::Message;
use trust_dns::rr::LowerName;

use super::dnsclient::DnsClient;
use super::config::DnsProxyConf;
use super::super::ruling::DomainMatcher;
use trust_dns::serialize::binary::BinDecoder;
use trust_dns::serialize::binary::BinDecodable;
use futures_cpupool::CpuPool;

type SF<V, E> = Future<Item=V, Error=E> + Send;

pub struct SmartResolver {
    region_resolver: Vec<(Bytes, DnsClient)>,
    default_resolver: DnsClient,
    router: Arc<DomainMatcher>,
}

impl SmartResolver {
    pub fn new(router: Arc<DomainMatcher>, regionconf: &DnsProxyConf, pool: CpuPool)
        -> Result<SmartResolver, Error> {
        let rresolvers: Vec<(Bytes, DnsClient)> = regionconf.resolv
            .iter().map(|(r, s)| {
            let dc = DnsClient::new(s, &pool);
            (r.clone(), dc)
        }).collect();
        let dresolver = DnsClient::new(&regionconf.default, &pool);
        Ok(SmartResolver {
            region_resolver: rresolvers,
            default_resolver: dresolver,
            router: router,
        })
    }

    pub fn handle_future(&self, buffer: &[u8]) -> Box<SF<Vec<u8>, Error>> {
        let mut decoder = BinDecoder::new(&buffer);
        let message = Message::read(&mut decoder).expect("msg deco err");

        let name = {
            let queries = message.queries();
            if queries.len() != 1 {
                return Box::new(future::err(format_err!("more than 1 queries in a message: {:?}", message)));
            }
            let q = &queries[0];
            LowerName::new(q.name())
        };

        let client = self.choose_resolver(&name);
        debug!("Dns query {:?} using {:?}", name, client);
        Box::new(client.resolve(buffer.to_vec())
            .map_err(|e| format_err!("resolve error: {:?}", e)))
    }

    fn choose_resolver(&self,name: &LowerName)-> &DnsClient {
        let n = name.to_string();
        let n: Vec<&str> = n.trim_right_matches('.').split('.').rev().collect();
        let d = n.join(".");
        let r = self.router.rule_domain(d.as_bytes());
        if let Some(region) = r {
            for &(ref reg, ref res) in &self.region_resolver {
                if  region.as_ref() == reg {
                    return &res
                }
            }
            warn!("no server found for {}", name);
        }
        return &self.default_resolver;
    }

}
