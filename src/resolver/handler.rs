use std::error::Error;
use std::sync::Arc;

use futures::Future;

use trust_dns::error::*;
use trust_dns::op::Message;
use trust_dns::rr::LowerName;

use super::dnsclient::DnsClient;
use super::config::DnsProxyConf;
use super::super::ruling::DomainMatcher;
use trust_dns::serialize::binary::BinDecoder;
use trust_dns::serialize::binary::BinDecodable;
use trust_dns_proto::xfer::DnsResponse;
use trust_dns_server::server::RequestHandler;
use trust_dns_server::server::ResponseHandler;
use trust_dns_server::server::Request;
use resolver::util::message_request_to_message;
use std::io;

pub struct SmartResolver {
    region_resolver: Vec<(String, DnsClient)>,
    default_resolver: DnsClient,
    router: Arc<DomainMatcher>,
}

impl RequestHandler for SmartResolver {
    fn handle_request<'q, 'a, R: ResponseHandler + 'static>(&'a self,
                                                            request: &Request,
                                                            response_handle: R)
        -> io::Result<()> {
        let m = &request.message;
        let m = message_request_to_message(m);
        let res = self.async_response(m, false)
            .map_err(|e| {
                io::Error::new(io::ErrorKind::Other, e)
            });
        let r1 =
            res.and_then(|res| {
                let mres: Message = res.into();
                response_handle.send(mres)
            })
                .map_err(|e| {
                    error!("Error handling request: {:?}", e);
                    ()
                });
        Ok(())
    }
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

    pub fn handle_future(&self, buffer: &[u8], use_tcp:bool)
        -> Box<Future<Item=DnsResponse, Error=String> + Send> {
        let mut decoder = BinDecoder::new(&buffer);
        let message = Message::read(&mut decoder).expect("msg deco err");
        self.async_response(message, use_tcp)
    }

    pub fn async_response(&self, message: Message, use_tcp: bool)
        -> Box<Future<Item=DnsResponse, Error=String> + Send> {
        let name = {
            let queries = message.queries();
            if queries.len() != 1 {
                panic!("more than 1 queries in a message: {:?}", message);
            }
            let q = &queries[0];
            LowerName::new(q.name())
        };
        let id = message.id();

        let client = self.choose_resolver(&name);
        let fr = client.resolve(
            message, &name, use_tcp);
        let f_id =
            fr.and_then(move|mut r| {
                r.set_id(id);
                Ok(r)
        }).map_err(|e| format!("resolv fut err: {:?}", e));
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
