extern crate bytes;
extern crate error_chain;
extern crate futures;
extern crate futures_cpupool;
#[macro_use]
extern crate log;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_proto;
extern crate tokio_service;
extern crate trust_dns;
extern crate trust_dns_server;

use std::io;
use std::str;
use bytes::BytesMut;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use futures_cpupool::CpuPool;

use tokio_io::codec::{Encoder, Decoder};

use tokio_proto::TcpServer;

mod proto;
mod resolver;
mod ruling;
mod service;

pub fn run(port: &str)-> Result<(), ()> {
    let config_path = "config";
    // Specify the localhost address
    let addr = Ipv4Addr::from_str("127.0.0.1").unwrap();
    let port = u16::from_str(port).unwrap();
    let socket = SocketAddr::new(IpAddr::V4(addr), port);

    let mut core = tokio_core::reactor::Core::new().map_err(|_| ())?;
    let pool = CpuPool::new(20);
    // The builder requires a protocol and an address
    let server = TcpServer::new(proto::LineProto,
                                socket);

    // We provide a way to *instantiate* the service for each new
    // connection; here, we just immediately return a new instance.
    let ruler = ruling::Ruler::new(config_path);
    let r = Arc::new(ruler);

    let future = pool.spawn_fn(move|| {
        server.serve(move || {
            let s = service::Echo::new(r.clone());
            Ok(s)
        });
        let res: Result<(), ()> = Ok(());
        res
    });
    let h = core.handle();
    resolver::start_resolver(h);
    core.run(future).map_err(|_| ())?;
    Ok(())
}

pub struct LineCodec;

impl Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<String>> {
        if let Some(i) = buf.iter().position(|&b| b == b'\n') {
            // remove the serialized frame from the buffer.
            let line = buf.split_to(i);

            // Also remove the '\n'
            buf.split_to(1);

            // Turn this data into a UTF string and return it in a Frame.
            match str::from_utf8(&line) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other,
                                             "invalid UTF-8")),
            }
        } else {
            Ok(None)
        }
    }
}

impl Encoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn encode(&mut self, msg: String, buf: &mut BytesMut) -> io::Result<()> {
        buf.extend(msg.as_bytes());
        buf.extend(b"\n");
        Ok(())
    }
}


