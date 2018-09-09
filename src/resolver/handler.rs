use std::net::SocketAddr;

use futures::future;
use futures::Future;

use tokio_core::reactor::Handle;
use trust_dns::error::*;
use trust_dns::op::message::Message;
use trust_dns::op::Query;
use trust_dns_server::server::Request;

use tokio_timer::Timer;
use std::time::Duration;

use super::dnsclient::DnsClient;

pub struct SmartResolver {
    dnsclient: DnsClient,
}

impl SmartResolver {
    pub fn new(sa: SocketAddr, handle: Handle)-> Result<SmartResolver, ClientError> {

        let c = DnsClient::new(sa, handle)?;
        Ok(SmartResolver {
            dnsclient: c,
        })
    }

    pub fn handle_future(&self, req: &Request, use_tcp:bool) -> Box<Future<Item=Message, Error=ClientError>> {
        let queries = req.message.queries();
        if queries.len() != 1 {
            panic!("more than 1 queries in a message: {:?}", req.message);
        }
        let q: &Query = &queries[0];
        let name = q.name();
        println!("querying {:?}", name);
        let id = req.message.id();

        let timer = Timer::default();
        let timeout = timer.sleep(Duration::from_secs(5));

        let res = self.dnsclient.resolve(q, use_tcp);
        let m
        = res.and_then(move|result| {
            let mut msg = Message::new();
            msg.set_id(id);
            for ans in result.answers() {
                msg.add_answer(ans.clone());
            }
            timeout.then(|_| future::ok(msg))
        });
        Box::new(m)
    }

}
