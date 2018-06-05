// Copyright 2015-2016 Benjamin Fry <benjaminfry@me.com>
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
use std;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use futures::{Future, Stream};

use tokio;
use trust_dns::udp::UdpStream;
use trust_dns::tcp::TcpStream;

use trust_dns_server::server::{TimeoutStream};

use super::handler::SmartResolver;

// TODO, would be nice to have a Slab for buffers here...

/// A Futures based implementation of a DNS server
pub struct ServerFuture {
    handler: Arc<SmartResolver>
}

impl ServerFuture {
    /// Creates a new ServerFuture with the specified Handler.
    pub fn new(handler: SmartResolver) -> io::Result<ServerFuture> {
        Ok(ServerFuture {
               handler: Arc::new(handler),
           })
    }

    /// Register a UDP socket. Should be bound before calling this function.
    pub fn listen_udp(&self, socket: tokio::net::UdpSocket){
        debug!("registered udp: {:?}", socket);

        // create the new UdpStream
        let (buf_stream, stream_handle) =
            UdpStream::with_bound(socket);
        let request_stream = RequestStream::new(buf_stream, stream_handle);
        let handler = self.handler.clone();

        // this spawns a ForEach future which handles all the requests into a Handler.
        let f = request_stream
            .for_each(move |(request, response_handle)| {
                let response = handler.handle_future(&request, false);
                let r = response.and_then(|res| {
                    let mut rh = response_handle;
                    let _ = rh.send(res).map_err(|_| 5);
                    Ok(())
                }).then(|_| Ok(()));
                tokio::spawn(r);
                Ok(())
            })
            .map_err(|e| debug!("error in UDP request_stream handler: {}", e));
        tokio::spawn(f);
    }

    /// Register a TcpListener to the Server. This should already be bound to either an IPv6 or an
    ///  IPv4 address.
    ///
    /// To make the server more resilient to DOS issues, there is a timeout. Care should be taken
    ///  to not make this too low depending on use cases.
    ///
    /// # Arguments
    /// * `listener` - a bound TCP socket
    /// * `timeout` - timeout duration of incoming requests, any connection that does not send
    ///               requests within this time period will be closed. In the future it should be
    ///               possible to create long-lived queries, but these should be from trusted sources
    ///               only, this would require some type of whitelisting.
    pub fn listen_tcp(&self,
                      listener: tokio::net::TcpListener,
                      timeout: Duration)-> Box<Future<Item=(), Error=()>> {
        // TODO: this is an awkward interface with socketaddr...
        debug!("registered tcp: {:?}", listener);

        // for each incoming request...
        let f = listener
            .incoming()
            .for_each(move |(tcp_stream, src_addr)| {
                debug!("accepted request from: {}", src_addr);
                // take the created stream...
                let (buf_stream, stream_handle) = TcpStream::from_stream(tcp_stream, src_addr);
                let timeout_stream =
                    TimeoutStream::new(buf_stream, timeout)
                        .map_err(|_| ()).unwrap();
                let request_stream = RequestStream::new(timeout_stream, stream_handle);
                let handler = self.handler.clone();

                // and spawn to the io_loop
                tokio::spawn(request_stream
                    .for_each(move |(request, response_handle)| {
                        let response = handler.handle_future(&request, true);
                        let f = response.and_then(move |r| {
                            let mut rh =  response_handle;
                            let _ = rh.send(r);
                            Ok(())
                        }).then(|_| Ok(()));
                        tokio::spawn(f);
                        Ok(())
                    })
                    .map_err(move |e| {
                        debug!("error in TCP request_stream src: {:?} error: {}",
                               src_addr,
                               e);
                    })
                );

                Ok(())
            }).map_err(|_| ());
        //.map_err(|e| debug!("error in inbound tcp_stream: {}", e))
        Box::new(f)
    }
}

