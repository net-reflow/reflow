use std::net::UdpSocket as UdpSocketStd;
use tokio::net::UdpSocket as UdpSocketTokio;
use std::net::SocketAddr;
use tokio::reactor::Handle;
use std::io;

use super::TIMEOUT;
use std::time::Duration;

pub async fn udp_get(addr: &SocketAddr, data: Vec<u8>)
    -> io::Result<Vec<u8>> {
    let uss = UdpSocketStd::bind("0.0.0.0:0")?;
    uss.set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;
    let ust = UdpSocketTokio::from_std(
        uss, &Handle::current())?;
    let (sock, mut buf) = await!(ust.send_dgram(data, addr))?;
    buf.resize(998, 0);
    let buf = vec![0; 998];
    let (_sock, mut buf,nb,_a) = await!(sock.recv_dgram(buf))?;
    buf.truncate(nb);
    Ok(buf)
}
