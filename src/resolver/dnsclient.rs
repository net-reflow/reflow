use std::net::SocketAddr;
use std::cell::RefCell;
use std::fmt;
use std::time;
use std::sync;

use futures::Future;
use tokio_core::reactor::Handle;
use trust_dns::error::*;
use trust_dns::rr::domain::Name;
use trust_dns::op::Message;
use trust_dns::udp::UdpClientStream;
use trust_dns::tcp::TcpClientStream;
use trust_dns::client::ClientFuture;
use trust_dns::client::BasicClientHandle;
use trust_dns::client::ClientHandle;

use super::monitor_failure::FailureCounter;

pub struct DnsClient {
    fut_client: RefCell<BasicClientHandle>,
    server: SocketAddr,
    handle: Handle,
    udp_fail_monitor: sync::Arc<FailureCounter>,
}

impl fmt::Display for DnsClient {
    fn fmt(&self, fmt: &mut fmt::Formatter)-> fmt::Result {
        write!(fmt, "{}", self.server)
    }
}

impl DnsClient {
    pub fn new(sa: SocketAddr, handle: Handle) -> Result<DnsClient, ClientError> {
        let (streamfut, streamhand) = UdpClientStream::new(sa, &handle.clone());
        let futclient = ClientFuture::with_timeout(streamfut, streamhand, &handle.clone(), time::Duration::from_secs(3), None);

        Ok(DnsClient {
            fut_client: RefCell::new(futclient),
            server: sa,
            handle: handle,
            udp_fail_monitor: sync::Arc::new(FailureCounter::new())
        })
    }

    fn get_tcp_client(&self)-> BasicClientHandle {
        let (streamfut, streamhand) = TcpClientStream::new(self.server, &self.handle.clone());
        let futtcp = ClientFuture::new(streamfut, streamhand, &self.handle.clone(), None);
        futtcp
    }

    pub fn resolve(&self, q: Message, name: &Name, require_tcp: bool)-> Box<Future<Item=Message, Error=ClientError>> {
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
            let udpres = self.fut_client.borrow_mut().send(q);
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
