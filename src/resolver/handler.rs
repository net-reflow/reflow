use std::sync::Arc;

use failure::Error;
use failure::err_msg;
use futures::Future;
use futures::future;

use trust_dns::error::*;
use trust_dns::op::Message;
use trust_dns::rr::LowerName;

use super::dnsclient::DnsClient;
use super::config::DnsProxyConf;
use super::super::ruling::DomainMatcher;
use trust_dns::serialize::binary::BinDecoder;
use trust_dns::serialize::binary::BinDecodable;
use trust_dns_proto::xfer::DnsResponse;

type SF<V, E> = Future<Item=V, Error=E> + Send;

pub struct SmartResolver {
    region_resolver: Vec<(String, DnsClient)>,
    default_resolver: DnsClient,
    router: Arc<DomainMatcher>,
}

impl SmartResolver {
    pub fn new(router: Arc<DomainMatcher>, regionconf: &DnsProxyConf)
        -> Result<SmartResolver, Error> {
        let rresolvers: Vec<(String, Result<DnsClient, ClientError>)> = regionconf.resolv
            .iter().map(|(r, s)| {
            let dc = DnsClient::new(s.clone());
            (r.clone(), dc)
        }).collect();
        if let Some(_) = rresolvers.iter().find(|&&(_, ref c)| c.is_err()) {
            return Err(err_msg("error while connecting with a dns server"));
        }
        let rresolvers = rresolvers.into_iter().map(|(region, cli)| (region, cli.unwrap())).collect();
        let dresolver = DnsClient::new(regionconf.default.clone())
            .map_err(|e| format_err!("client error {}", e))?;
        Ok(SmartResolver {
            region_resolver: rresolvers,
            default_resolver: dresolver,
            router: router,
        })
    }

    pub fn handle_future(&self, buffer: &[u8], use_tcp:bool) -> Box<SF<DnsResponse, Error>> {
        let mut decoder = BinDecoder::new(&buffer);
        let message = Message::read(&mut decoder).expect("msg deco err");
        self.async_response(message, use_tcp)
    }

    pub fn async_response(&self, message: Message, use_tcp: bool) -> Box<SF<DnsResponse, Error>> {
        let name = {
            let queries = message.queries();
            if queries.len() != 1 {
                return Box::new(future::err(format_err!("more than 1 queries in a message: {:?}", message)));
            }
            let q = &queries[0];
            LowerName::new(q.name())
        };
        let id = message.id();

        let client = self.choose_resolver(&name);
        let fr = client.resolve(
            message, &name, use_tcp).map_err(|e| format_err!("resolve error {}", e));
        let f_id =
            fr.and_then(move|mut r| {
                r.set_id(id);
                Ok(r)
        });
        Box::new(f_id)
    }

    fn choose_resolver(&self,name: &LowerName)-> &DnsClient {
        let n = name.to_string();
        let n: Vec<&str> = n.trim_right_matches('.').split('.').rev().collect();
        let d = n.join(".");
        let r = self.router.rule_domain(&d);
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
