use std::io;
use std::net::SocketAddr;
use std::net::UdpSocket as UdpSocketStd;
use tokio::net::UdpSocket as UdpSocketTokio;

use super::TIMEOUT;
use std::time::Duration;
use tokio_net::driver::Handle;

pub async fn udp_get(addr: &SocketAddr, data: Vec<u8>) -> io::Result<Vec<u8>> {
    let uss = UdpSocketStd::bind("0.0.0.0:0")?;
    uss.set_read_timeout(Some(Duration::from_secs(TIMEOUT)))?;
    let mut ust = UdpSocketTokio::from_std(uss)?;
    ust.send_to(&data, addr).await?;
    let mut buf = data;
    buf.resize(998, 0);
    let (nb, _a) = ust.recv_from(&mut buf).await?;
    buf.truncate(nb);
    Ok(buf)
}
