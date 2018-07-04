use proto::socks::listen::listen;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use failure::Error;
use futures::{Stream, Future, future, future::Either};
use tokio;
use tokio_io::io::copy;
use tokio_io::AsyncRead;
use proto::socks::listen::Incoming;
use proto::socks::{Command, SocksError};
use proto::socks::Address;
use tokio::io::AsyncWrite;

pub fn listen_socks(addr: &SocketAddr)->Result<(), Error >{
    let fut =
        listen(addr)?.for_each(|s| {
            let f =
                s.and_then(|i| handle_incoming(i))
                    .map_err(|e| error!("error handling client {:?}", e));
            tokio::spawn(f);
            Ok(())
        }).map_err(|e| error!("Listen error {:?}", e));
    tokio::spawn(fut);
    Ok(())
}

fn handle_incoming(incom: Incoming)-> impl Future<Item=(), Error=Error> {
    let c = incom.req.command;
    let a = match get_addr(c, &incom.req.address) {
        Ok(a) => a,
        Err(e) => return Either::A(future::err(e)),
    };
    let s = tokio::net::TcpStream::connect(&a);
    let f = s.and_then(|stream| {
        let (ur, uw) = stream.split();
        let (cr, cw) = incom.stream.split();
        tokio::spawn(run_copy(ur, cw));
        tokio::spawn(run_copy(cr, uw));
        Ok(())
    }).map_err(|e| e.into());
    Either::B(f)
}

fn run_copy<R, W>(reader: R, writer: W) -> impl Future<Item=(), Error=()>
    where R: AsyncRead,
          W: AsyncWrite, {
    copy(reader, writer).map(|_x| ())
        .map_err(|e| error!("Error copying {:?}", e))
}

fn get_addr(c: Command, addr: &Address)-> Result<SocketAddr, Error> {
    if c != Command::TcpConnect {
        return Err(SocksError::CommandUnSupport { cmd: c as u8}.into());
    }
    let mut addrs = addr.to_socket_addrs()?;
    let a = addrs.nth(0).ok_or(format_err!("No address for {:?}", addr))?;
    Ok(a)
}