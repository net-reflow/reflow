#### socks implemented with async and await!

---

This implementation provides a quite flexible and easy to
use option for common use cases. 
Instead of creating `struct`s for handling socks streams,
it just uses `tokio::net::TcpStream` and provides one 
function that handles the handshake for you.

##### Client

There is a client in `examples`, try it with `cargo run --example  client`.

Here is how it works:

    let mut tcpStream = await!(TcpStream::connect(&socks_addr))?;

First you connect to the socks server using tokio.
    
    await!(connect_socks_to(&mut tcpStream, Address::SocketAddress(remote_website)))?;

Then you call `connect_socks_to`, which handles the handshake and instructs the server 
to connect to the website you want to communicate with.

Now you've got a `TcpStream` that acts just like a connection to the website.

With this design, it should be straightforward to chain several socks servers, although
more tests should be carried out.

##### Server

Run the example with `cargo run --example server`.
The server has a similar design, you just use tokio to create a tcp listener:

    tokio::net::TcpListener::bind(&addr)?;
    
And when you get a connection, you again call one function to do the handshake:

    let (client_stream, request) = await!(handle_socks_handshake(client_stream))?;    
    
##### Other features

Sending and receiving udp packets is implemented.

Authentication is not implemented.

Other less common features are not implemented.
