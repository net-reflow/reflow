use std::io;
use std::net::{ SocketAddr, IpAddr};

use tokio_core::reactor::Handle;
use tokio_core::net::TcpListener;
use tokio_service::{Service, NewService};
use tokio_io::AsyncRead;
use futures::{ Future, Stream, Sink};

use super::super::LineCodec;

pub fn rule_listen<S>(s: S, port: u16, handle: Handle) -> io::Result<Box<Future<Item=(), Error=()>>>
    where S: NewService<Request = String,
                        Response = String,
                        Error = io::Error> + 'static
{

    let socket = SocketAddr::new(IpAddr::from([127,0,0,1]), port);

    let listener = TcpListener::bind(&socket, &handle)?;

    let connections = listener.incoming();
    let server = connections.for_each(move |(socket, _peer_addr)| {
        let (writer, reader) = socket.framed(LineCodec).split();
        let service = s.new_service()?;

        let responses = reader.and_then(move |req| service.call(req));
        let server = writer.send_all(responses)
            .then(|_| Ok(()));
        handle.spawn(server);

        Ok(())
    }).map_err(|_| ());

    Ok(Box::new(server))
}
