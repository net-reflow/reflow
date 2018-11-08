mod udp;
use failure::Error;

use crate::resolver::serve::udp::UdpListener;
use futures_cpupool::CpuPool;
use tokio;
use std::sync::Arc;

use crate::conf::DomainMatcher;
use crate::resolver::handler;
use crate::conf::DnsProxy;
use crate::resolver::serve::udp::UdpMsg;
use crate::resolver::handler::SmartResolver;
use tokio_async_await::stream::StreamExt;

pub fn serve(conf: DnsProxy, matcher: Arc<DomainMatcher>, pool: CpuPool) -> Result<(), Error> {
    let handler = handler::SmartResolver::new(matcher, &conf, pool)?;
    let handler = Arc::new(handler);
    let addr = conf.listen;
    let mut listen = UdpListener::bind(&addr)?;
    tokio::spawn_async(  async move{
        while let Some(n) = await!(listen.next()) {
            match n {
                Ok(u) => {
                    let h = (&handler).clone();
                    tokio::spawn_async(async {
                        if let Err(e) = await!(handle_dns_client(u, h)) {
                            error!("Error handling dns client: {:?}", e);
                        }
                    });
                },
                Err(e) => warn!("error getting next dns client: {:?}", e),
            }
        }
    });
    Ok(())
}

async fn handle_dns_client(u: UdpMsg, handler: Arc<SmartResolver>)-> Result<(), Error> {
    let x = await!(handler.handle_future( & u.data))?;
    await!(u.sock.send_dgram( & x, & u.peer))?;
    Ok(())
}