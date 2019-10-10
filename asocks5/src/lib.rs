#![feature(async_await)]

mod client;
mod codec;
mod consts;
mod heads;
pub mod listen;
pub mod socks;

pub use self::client::connect_socks_socket_addr;
pub use self::client::connect_socks_to;
pub use self::client::udp::Socks5Datagram;
pub use self::consts::Command;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
