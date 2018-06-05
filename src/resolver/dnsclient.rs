use std::net::SocketAddr;
use std::cell::RefCell;
use std::fmt;
use std::time;
use std::sync;

use futures::Future;
use trust_dns::error::*;
use trust_dns::rr::domain::Name;
use trust_dns::rr::LowerName;
use trust_dns::op::Message;
use trust_dns::udp::UdpClientStream;
use trust_dns::tcp::TcpClientStream;
use trust_dns::client::ClientFuture;
use trust_dns::client::BasicClientHandle;
use trust_dns::client::ClientHandle;
use trust_dns_proto::xfer::dns_handle::DnsHandle;

use super::monitor_failure::FailureCounter;
use std::io;

pub struct DnsClient {
    server: SocketAddr,
    udp_fail_monitor: sync::Arc<FailureCounter>,
}

impl fmt::Display for DnsClient {
    fn fmt(&self, fmt: &mut fmt::Formatter)-> fmt::Result {
        write!(fmt, "{}", self.server)
    }
}

impl DnsClient {
    pub fn new(sa: SocketAddr) -> Result<DnsClient, ClientError> {
        Ok(DnsClient {
            server: sa,
            udp_fail_monitor: sync::Arc::new(FailureCounter::new())
        })
    }

    fn get_udp_client(&self)-> Box<Future<Item=BasicClientHandle, Error=ClientError>> {
        let (streamfut, streamhand) =
            UdpClientStream::new(self.server);
        let futclient = ClientFuture::with_timeout(
            streamfut, streamhand,
            time::Duration::from_secs(3), None);
        futclient
    }

    fn get_tcp_client(&self)-> Box<futures::Future<Error=trust_dns::error::Error, Item=trust_dns::client::BasicClientHandle>>  {
        let (streamfut, streamhand) = TcpClientStream::new(self.server);
        let futtcp = ClientFuture::new(streamfut, streamhand, None);
        futtcp
    }

    pub fn resolve(&self, q: Message, name: &LowerName, require_tcp: bool)-> Box<Future<Item=Message, Error=ClientError>> {
        let res = if require_tcp {
            println!("{} using {}, tcp required by client", name, self.server);
            let mut c = self.get_tcp_client();
            c.send(q)
        } else if  self.udp_fail_monitor.should_wait() {
            println!("{} using {}, fallback to tcp", name, self.server);
            let mut c = self.get_tcp_client();
            c.send(q)
        } else {
            println!("{} using {}, via udp", name, self.server);
            let udpres
            = self.get_udp_client().and_then(|c| c.send(q));
            let mon = self.udp_fail_monitor.clone();
            let res = udpres.then(move|x| {
                match x {
                    Ok(v) => {
                        mon.log_success();
                        Ok(v)
                    }
                    Err(e) => {
                        mon.log_failure();
                        Err(e)
                    }
                }
            });
            Box::new(res)
        };
        res
    }
}
