mod udp;
use failure::Error;

use resolver::serve::udp::UdpListener;
use futures::Stream;
use futures::Future;
use futures_cpupool::CpuPool;
use tokio;
use std::sync::Arc;

use ruling::DomainMatcher;
use resolver::config::DnsProxyConf;
use resolver::handler;

pub fn serve(router: Arc<DomainMatcher>, conf: DnsProxyConf, pool: CpuPool) -> Result<(), Error> {
    let handler = handler::SmartResolver::new(router, &conf, pool)?;
    let addr = conf.listen;
    let listen = UdpListener::bind(&addr)?;
    let f = listen
        .for_each(move |u| {
            tokio::spawn(
                handler.handle_future(&u.data)
                    .and_then(move |x| {
                        let mut u = u;
                        u.sock.poll_send_to(&x, &u.peer).map_err(|e| e.into())
                    })
                    .map(|_s| ())
                    .map_err(|e| warn!("erro: {}", e))
            );
            Ok(())
        })
        .map_err(|e| warn!("error: {:?}", e));
    tokio::spawn(f);
    Ok(())
}
