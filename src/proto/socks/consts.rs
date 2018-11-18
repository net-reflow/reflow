
use std::convert::TryFrom;

use super::SocksError;

pub const SOCKS5_VERSION:                          u8 = 0x05;

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum AuthMethod {
    NONE = 0x00,
    GSSAPI = 0x01,
    PASSWORD = 0x02,
    NotAcceptable = 0xff,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    TcpConnect = 0x01,
    TcpBind = 0x02,
    UdpAssociate = 0x03,
}

impl TryFrom<u8> for Command {
    type Error = SocksError;

    fn try_from(value: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match value {
            1 => Ok(Command::TcpConnect),
            2 => Ok(Command::TcpBind),
            3 => Ok(Command::UdpAssociate),
            v => Err(SocksError::CommandUnSupport { cmd: v }),
        }
    }
}

#[derive(Clone, Copy)]
pub enum AddrType {
    IPV4 = 0x01,
    DomainName = 0x03,
    IPV6 = 0x04,
}

impl TryFrom<u8> for AddrType {
    type Error = SocksError;

    fn try_from(value: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match value {
            1 => Ok(AddrType::IPV4),
            3 => Ok(AddrType::DomainName),
            4 => Ok(AddrType::IPV6),
            v => Err(SocksError::AddressTypeNotSupported {code: v})
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Reply {
    SUCCEEDED = 0x00,
    GeneralFailure = 0x01,
    ConnectionNotAllowed = 0x02,
    NetworkUnreachable = 0x03,
    HostUnreachable = 0x04,
    ConnectionRefused = 0x05,
    TtlExpired = 0x06,
    CommandNotSupported = 0x07,
    AddressTypeNotSupported = 0x08,
}

impl TryFrom<u8> for Reply {
    type Error = SocksError;

    fn try_from(value: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        use self::Reply::*;
        let r = match value {
            0 => SUCCEEDED,
            1 => GeneralFailure,
            2 => ConnectionNotAllowed,
            3 => NetworkUnreachable,
            4 => HostUnreachable,
            5 => ConnectionRefused,
            6 => TtlExpired,
            7 => CommandNotSupported ,
            8 => AddressTypeNotSupported,
            x => return Err(SocksError::InvalidReply { reply: x})
        };
        Ok(r)
    }
}