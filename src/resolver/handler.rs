use std::error::Error;
use std::sync::Arc;

use futures::Future;

use tokio_core::reactor::Handle;
use trust_dns::error::*;
use trust_dns::op::message::Message;
use trust_dns::op::Query;
use trust_dns::rr::domain::Name;
use trust_dns_server::server::Request;

use super::dnsclient::DnsClient;
use super::config::RegionalResolvers;
use super::super::ruling::Ruler;

pub struct SmartResolver {
    region_resolver: Vec<(String, DnsClient)>,
    default_resolver: DnsClient,
    router: Arc<Ruler>,
}

impl SmartResolver {
    pub fn new(handle: Handle, router: Arc<Ruler>)-> Result<SmartResolver,
        Box<Error>> {

        let regionconf = RegionalResolvers::new("config/resolve.config".into())?;
        let rresolvers: Vec<(String, Result<DnsClient, ClientError>)> = regionconf.resolv
            .into_iter().map(|(r, s)| {
            let dc = DnsClient::new(s, handle.clone());
            (r, dc)
        }).collect();
        if let Some(_) = rresolvers.iter().find(|&&(_, ref c)| c.is_err()) {
            bail!("error while connecting with a dns server")
        }
        let rresolvers = rresolvers.into_iter().map(|(region, cli)| (region, cli.unwrap())).collect();
        let dresolver = DnsClient::new(regionconf.default, handle.clone())?;
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
        let q: &Query = &queries[0];
        let name = q.name();

        let client = self.choose_resolver(name);
        println!("using {} for {}", client, name);
        let f = client.resolve(req.message.clone(), use_tcp);
        f
    }

    fn choose_resolver(&self,name: &Name)-> &DnsClient {
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
