use std::net::SocketAddr;

use trust_dns::error::*;
use trust_dns::op::message::Message;
use trust_dns::op::Query;
use trust_dns::udp::UdpClientConnection;
use trust_dns::client::{Client, SyncClient};
use trust_dns_server::server::Request;

pub struct SmartResolver {
    conn: SyncClient,
}

impl SmartResolver {
    pub fn new(sa: SocketAddr)-> SmartResolver {
        let client = UdpClientConnection::new(sa);
        let client: UdpClientConnection = client.unwrap();
        let client =SyncClient::new(client);
        SmartResolver {
            conn: client,
        }
    }

    pub fn handle_request(&self, req: &Request) -> Message {
        let queries = req.message.queries();
        if queries.len() > 1 {
            debug!("more than 1 queries in a message: {:?}", req.message);
        }
        if queries.len() < 1 {
            panic!("empty query");
        }
        let q: &Query = &queries[0];
        let name = q.name();
        println!("querying {:?}", name);

        let result: ClientResult<Message> = self.conn.query(
            name, q.query_class(), q.query_type());
        let result: Message = result.unwrap();
        let mut msg = Message::new();
        msg.set_id(req.message.id());
        for ans in result.answers() {
            msg.add_answer(ans.clone());
        }
        msg
    }
}
