use std::net::SocketAddr;
use std::fmt;
use std::time;
use std::sync;

use futures::Future;
use trust_dns::error::*;
use trust_dns::rr::LowerName;
use trust_dns::op::Message;
use trust_dns::udp::UdpClientStream;
use trust_dns::tcp::TcpClientStream;
use trust_dns::client::ClientFuture;
use trust_dns::client::BasicClientHandle;
use trust_dns_proto::xfer::dns_handle::DnsHandle;
use trust_dns_proto::xfer::DnsResponse;

use super::monitor_failure::FailureCounter;
use trust_dns_proto::xfer::DnsRequest;

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

    fn get_udp_client(&self)-> Box<Future<Item=BasicClientHandle, Error=ClientError> + Send> {
        let (streamfut, streamhand) =
            UdpClientStream::new(self.server);
        let futclient = ClientFuture::with_timeout(
            streamfut, streamhand,
            time::Duration::from_secs(3), None);
        futclient
    }

    fn get_tcp_client(&self)-> Box<Future<Error=ClientError, Item=BasicClientHandle> + Send>  {
        let (streamfut, streamhand) = TcpClientStream::new(self.server);
        let futtcp = ClientFuture::new(streamfut, streamhand, None);
        futtcp
    }

    pub fn resolve(&self, q: Message, name: &LowerName, require_tcp: bool)
        -> Box<Future<Item=DnsResponse, Error=ClientError> + Send> {
        let req :DnsRequest = q.into();
        if require_tcp {
            println!("{} using {}, tcp required by client", name, self.server);
            let c: Box<Future<Item=_,Error=_>+Send> = self.get_tcp_client();
            return Box::new(c.and_then(|mut h| h.send(req)));
        } else if self.udp_fail_monitor.should_wait() {
            println!("{} using {}, fallback to tcp", name, self.server);
            let c = self.get_tcp_client();
            return Box::new(c.and_then(|mut h| h.send(req)));
        } else {
            println!("{} using {}, via udp", name, self.server);
            let udpres
            = self.get_udp_client().and_then(|mut c| c.send(req));
            let mon = self.udp_fail_monitor.clone();
            let res =
                udpres.then(move |then | match then {
                    Ok(v) => {
                        mon.log_success();
                        Ok(v)
                    }
                    Err(e) => {
                        mon.log_failure();
                        Err(e)
                    }
                });
            return Box::new(res);
        }
    }
}
