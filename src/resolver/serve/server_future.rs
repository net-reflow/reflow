// Copyright 2015-2016 Benjamin Fry <benjaminfry@me.com>
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
use std::io;
use std::sync::Arc;
use std::time::Duration;

use futures::{Future, Stream};

use tokio;
use trust_dns::udp::UdpStream;
use trust_dns::tcp::TcpStream;

use trust_dns_server::server::{TimeoutStream};
use trust_dns_server::server::ResponseHandle;
use trust_dns_server::server::ResponseHandler;

use super::super::handler::SmartResolver;
use trust_dns::op::Message;
use std::net::SocketAddr;
use trust_dns::BufStreamHandle;
use trust_dns::error::ClientError;

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
    pub fn listen_udp(&self, socket: tokio::net::UdpSocket) {
        debug!("registered udp: {:?}", socket);

        let (buf_stream, stream_handle) =
            UdpStream::with_bound(socket);
        let handler = self.handler.clone();

        let f = buf_stream
            .for_each(move |(request, peer)| {
                Self::handle_request(request, peer, stream_handle.clone(), handler.clone());
                Ok(())
            })
            .map_err(|e| debug!("error in UDP request_stream handler: {}", e));
        tokio::spawn(f);
    }

    /// Register an already bound  cpListener to the Server
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
                      timeout: Duration)-> io::Result<()> {
        debug!("registered tcp: {:?}", listener);
        let handler = self.handler.clone();
        let f = listener
            .incoming()
            .for_each(move |tcp_stream| {
                let src_addr = tcp_stream.peer_addr().unwrap();
                debug!("accepted tcp request from: {}", src_addr);
                let (buf_stream, stream_handle) =
                    TcpStream::from_stream(tcp_stream, src_addr);
                let timeout_stream =
                    TimeoutStream::new(buf_stream, timeout);
                let handler = handler.clone();

                tokio::spawn(timeout_stream
                    .for_each(move |(buf, peer)| {
                        Self::handle_request(buf, peer, stream_handle.clone(), handler.clone());
                        Ok(())
                    })
                    .map_err(move |e| {
                        debug!("error in TCP request_stream src: {:?} error: {:?}",
                               src_addr,
                               e);
                    })
                );

                Ok(())
            })
        .map_err(|e| error!("error in inbound tcp_stream: {}", e));
        tokio::spawn(f);
        Ok(())
    }

    fn handle_request(
        buffer: Vec<u8>,
        src_addr: SocketAddr,
        stream_handle: BufStreamHandle<ClientError>,
        handler: Arc<SmartResolver>,
    ) {
        let response = handler.handle_future(
            &buffer, false);
        let reply =
            response.and_then(move |res| {
                let reshand = ResponseHandle::new(src_addr, stream_handle);
                let mes: Message = res.into();
                reshand.send(mes).map_err(|e| e.into())
            }).map_err(|e| error!("Error replying dns: {:?}", e));
        tokio::spawn(reply);
    }
}

