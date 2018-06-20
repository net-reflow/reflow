use std::net::UdpSocket as UdpSocketStd;
use tokio::net::UdpSocket as UdpSocketTokio;
use std::net::SocketAddr;
use tokio::reactor::Handle;
use futures::Future;
use std::io;

use super::TIMEOUT;
use std::time::Duration;

pub fn udp_get(addr: &SocketAddr, data: Vec<u8>)
    -> io::Result<impl Future<Item=Vec<u8>,Error=io::Error> + Send> {
    let uss = UdpSocketStd::bind("0.0.0.0:0")?;
    uss.set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;
    let ust = UdpSocketTokio::from_std(
        uss, &Handle::current())?;
    let r =
        ust.send_dgram(data, addr)
            .and_then(|(sock, _buf)| {
                let buf = vec![0; 998];
                sock.recv_dgram(buf)
            })
            .and_then(|(_sock,buf,nb,_a)| {
                let mut buf =  buf;
                buf.truncate(nb);
                Ok(buf)
            });
    Ok(r)
}
