use std::error::Error;
use std::sync::Arc;

use futures::Future;

use tokio;
use trust_dns::error::*;
use trust_dns::op::Message;
use trust_dns::op::Query;
use trust_dns::rr::domain::Name;
use trust_dns::rr::LowerName;
use trust_dns_server::server::Request;

use super::dnsclient::DnsClient;
use super::config::DnsProxyConf;
use super::super::ruling::DomainMatcher;

pub struct SmartResolver {
    region_resolver: Vec<(String, DnsClient)>,
    default_resolver: DnsClient,
    router: Arc<DomainMatcher>,
}

impl SmartResolver {
    pub fn new(router: Arc<DomainMatcher>, regionconf :&DnsProxyConf)-> Result<SmartResolver,
        Box<Error>> {

        let rresolvers: Vec<(String, Result<DnsClient, ClientError>)> = regionconf.resolv
            .iter().map(|(r, s)| {
            let dc = DnsClient::new(s.clone());
            (r.clone(), dc)
        }).collect();
        if let Some(_) = rresolvers.iter().find(|&&(_, ref c)| c.is_err()) {
            bail!("error while connecting with a dns server")
        }
        let rresolvers = rresolvers.into_iter().map(|(region, cli)| (region, cli.unwrap())).collect();
        let dresolver = DnsClient::new(regionconf.default.clone())?;
        Ok(SmartResolver {
            region_resolver: rresolvers,
            default_resolver: dresolver,
            router: router,
        })
    }

    pub fn handle_future(&self, req: &Request, use_tcp:bool) -> Box<Future<Item=Message, Error=ClientError>> {
        let queries = req.message.queries();
        if queries.len() != 1 {
            panic!("more than 1 queries in a message: {:?}", req.message);
        }
        let q = &queries[0];
        let name = q.name();
        let id = req.message.id();

        let client = self.choose_resolver(name);
        let f = client.resolve(req.message.clone(), name, use_tcp);
        let f = f.and_then(move|mut r: Message| {
            r.set_id(id);
            Ok(r)
        });
        Box::new(f)
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
