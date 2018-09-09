extern crate bytes;
#[macro_use]
extern crate error_chain;
extern crate futures;
#[macro_use]
extern crate log;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_service;
extern crate trust_dns;
extern crate toml;
extern crate trust_dns_server;
#[macro_use]
extern crate serde_derive;
extern crate serde;

use std::io;
use std::str;
use bytes::BytesMut;
use std::str::FromStr;
use std::sync::Arc;

use tokio_io::codec::{Encoder, Decoder};

mod resolver;
mod ruling;
mod service;

pub fn run(port: &str)-> io::Result<()> {
    let config_path = "config";
    // Specify the localhost address
    let port = u16::from_str(port).unwrap();

    let mut core = tokio_core::reactor::Core::new()?;

    // We provide a way to *instantiate* the service for each new
    // connection; here, we just immediately return a new instance.
    let ruler = ruling::DomainMatcher::new(config_path)?;
    let ip_matcher = ruling::IpMatcher::new(config_path)?;
    let d = Arc::new(ruler);
    let i = Arc::new(ip_matcher);

    let h = core.handle();
    let r1 = d.clone();
    let i1 = i.clone();
    let rlisten =  ruling::serve::rule_listen(move| | Ok(service::Echo::new(r1.clone(), i1.clone())), port, h.clone()).unwrap();
    resolver::start_resolver(h, d.clone());
    core.run(rlisten).map_err(|_| io::Error::new(io::ErrorKind::Other, "tokio core start fail") )?;
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


