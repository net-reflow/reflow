#![feature(await_macro, async_await)]
#![feature(futures_api)]

mod consts;
pub mod listen;
mod heads;
mod codec;
mod client;
pub mod socks;

pub use self::consts::Command;
pub use self::client::connect_socks_to;
pub use self::client::udp::Socks5Datagram;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
