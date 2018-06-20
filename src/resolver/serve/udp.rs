//! Serve udp requests

use std::io;
use std::net::SocketAddr;
use std::convert::From;
use std::net::UdpSocket as UdpSocketStd;

use tokio::net::UdpSocket as UdpSocketTokio;
use tokio::reactor::Handle;
use futures::{Stream};
use futures::Async;


pub struct UdpListener {
    socket_std: UdpSocketStd,
    socket_tokio: UdpSocketTokio,
    buf: Vec<u8>,
}

impl UdpListener {
    pub fn bind(addr: &SocketAddr) -> io::Result<UdpListener> {
        let ss = UdpSocketStd::bind(addr)?;
        let ss_1 = ss.try_clone()?;
        let sock = UdpSocketTokio::from_std(
            ss, &Handle::current())?;
        let buf = vec![0; 998];
        Ok(UdpListener {
            socket_std: ss_1,
            socket_tokio: sock,
            buf: buf,
        })
    }
}

impl Stream for UdpListener {
    type Item = UdpMsg;
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Option<<Self as Stream>::Item>>, <Self as Stream>::Error> {
        loop {
            let r = self.socket_tokio.poll_recv_from(&mut self.buf);
            let (size, peer) = try_ready!(r);
            let mut newbuf = vec![0; size];
            newbuf.copy_from_slice(&self.buf[..size]);
            let newsock = self.socket_std.try_clone()?;
            let newsock = UdpSocketTokio::from_std(newsock, &Handle::current())?;
            let u = UdpMsg {
                sock: newsock,
                peer: peer,
                data: newbuf,
            };
            return Ok(Async::Ready(Some(u)));
        }
    }
}

pub struct UdpMsg {
    pub sock: UdpSocketTokio,
    pub peer: SocketAddr,
    pub data: Vec<u8>,
}