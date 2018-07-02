use proto::socks::listen::listen;
use std::net::SocketAddr;
use failure::Error;
use futures::{Stream, Future};
use tokio;

pub fn listen_socks(addr: &SocketAddr)->Result<(), Error >{
    let fut =
        listen(addr)?.for_each(|s| {
            let f =
                s.map(|st| ())
                    .map_err(|e| error!("error handling client {:?}", e));
            tokio::spawn(f);
            Ok(())
        }).map_err(|e| error!("Listen error {:?}", e));
    tokio::spawn(fut);
    Ok(())
}
