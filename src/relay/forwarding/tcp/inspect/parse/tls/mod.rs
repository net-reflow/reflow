use nom::{be_u8,be_u16, be_u24, be_u32, ErrorKind};

use std::fmt;
mod sni;

use bytes::Bytes;
use self::sni::{parse_tls_extension, TlsExtension};

/// Tls connection that can be recognized to some extent
#[derive(Clone, Debug)]
pub struct TlsWithSni {
    /// only possibly useful fields are included here
    pub version: TlsVersion,
    pub sni: Bytes,
}

pub fn parse_tls_sni(bs: &[u8])-> Option<TlsWithSni> {
    let x = parse_tls_plaintext(bs).ok().map(|y| y.1)?;
    let v = x.version;
    let n = x.get_sni()?;
    Some(TlsWithSni { version: v, sni: n })
}
/// Content type, as defined in IANA TLS ContentType registry
const TLS_RECORD_TYPE_HANDSHAKE: u8 = 0x16;

/// Handshake type
///
/// Handshake types are defined in [RFC5246](https://tools.ietf.org/html/rfc5246) and
/// the [IANA HandshakeType
/// Registry](https://www.iana.org/assignments/tls-parameters/tls-parameters.xhtml#tls-parameters-7)
const TLS_HANDSHAKE_TYPE_CLIENT_HELLO: u8 = 0x01;

/// TLS version
///
/// Only the TLS version defined in the TLS message header is meaningful, the
/// version defined in the record should be ignored or set to TLS 1.0
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TlsVersion(pub u16);

#[allow(non_upper_case_globals)]
impl TlsVersion {
    pub const Ssl30        : TlsVersion = TlsVersion(0x0300);
    pub const Tls10        : TlsVersion = TlsVersion(0x0301);
    pub const Tls11        : TlsVersion = TlsVersion(0x0302);
    pub const Tls12        : TlsVersion = TlsVersion(0x0303);
    pub const Tls13        : TlsVersion = TlsVersion(0x0304);

    pub const Tls13Draft18 : TlsVersion = TlsVersion(0x7f12);
    pub const Tls13Draft19 : TlsVersion = TlsVersion(0x7f13);
    pub const Tls13Draft20 : TlsVersion = TlsVersion(0x7f14);
    pub const Tls13Draft21 : TlsVersion = TlsVersion(0x7f15);
    pub const Tls13Draft22 : TlsVersion = TlsVersion(0x7f16);
    pub const Tls13Draft23 : TlsVersion = TlsVersion(0x7f17);
}

impl From<TlsVersion> for u16 {
    fn from(v: TlsVersion) -> u16 { v.0 }
}

impl fmt::Debug for TlsVersion {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TlsVersion::Ssl30        => fmt.write_str("TlsVersion::Ssl30"),
            TlsVersion::Tls10        => fmt.write_str("TlsVersion::Tls10"),
            TlsVersion::Tls11        => fmt.write_str("TlsVersion::Tls11"),
            TlsVersion::Tls12        => fmt.write_str("TlsVersion::Tls12"),
            TlsVersion::Tls13        => fmt.write_str("TlsVersion::Tls13"),
            TlsVersion::Tls13Draft18 => fmt.write_str("TlsVersion::Tls13Draft18"),
            TlsVersion::Tls13Draft19 => fmt.write_str("TlsVersion::Tls13Draft19"),
            TlsVersion::Tls13Draft20 => fmt.write_str("TlsVersion::Tls13Draft20"),
            TlsVersion::Tls13Draft21 => fmt.write_str("TlsVersion::Tls13Draft21"),
            TlsVersion::Tls13Draft22 => fmt.write_str("TlsVersion::Tls13Draft22"),
            TlsVersion::Tls13Draft23 => fmt.write_str("TlsVersion::Tls13Draft23"),
            _                        => write!(fmt, "{:x}", self.0),
        }
    }
}

impl fmt::LowerHex for TlsVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

/// TLS Client Hello (from TLS 1.0 to TLS 1.2)
///
/// Some fields are unparsed (for performance reasons), for ex to parse `ext`,
/// call the `parse_tls_extensions` function.
#[derive(Clone, Debug, PartialEq)]
pub struct TlsClientHello<'a> {
    /// TLS version of message
    pub version: TlsVersion,
    pub rand_time: u32,
    pub rand_data: &'a[u8],
    pub session_id: Option<&'a[u8]>,
    /// A list of ciphers supported by client
    pub ciphers: &'a[u8],
    /// A list of compression methods supported by client
    pub comp: &'a[u8],
    pub ext: Vec<TlsExtension<'a>>,
}

impl<'a> TlsClientHello<'a> {
    pub fn new(v:u16,rt:u32,rd:&'a[u8],sid:Option<&'a[u8]>,
               c:&'a[u8], co:&'a[u8],
               e: Vec<TlsExtension<'a>>
    ) -> Self {
        TlsClientHello {
            version: TlsVersion(v),
            rand_time: rt,
            rand_data: rd,
            session_id: sid,
            ciphers: c,
            comp: co,
            ext: e,
        }
    }

    #[allow(dead_code)]
    pub fn get_sni(self)-> Option<Bytes> {
        for e in self.ext {
            match e {
                TlsExtension::SNI(names) => {
                    for (nt, nb) in names {
                        if nt == 0 {
                            return Some(nb);
                        }
                    }
                }
                _ => {}
            }
        }
        return None;
    }
}



/// Helper macro for nom parsers: raise error if the condition is false
macro_rules! error_if (
  ($i:expr, $cond:expr, $err:expr) => (
    {
      if $cond {
        Err(::nom::Err::Error(error_position!($i, $err)))
      } else {
        Ok(($i, ()))
      }
    }
  );
);

named!(parse_tls_handshake_msg_client_hello<TlsClientHello>,
    do_parse!(
        v:         be_u16  >>
        rand_time: be_u32 >>
        rand_data: take!(28) >> // 28 as 32 (aligned) - 4 (time)
        sidlen:    be_u8 >> // check <= 32, can be 0
                   error_if!(sidlen > 32, ErrorKind::Custom(128)) >>
        sid:       cond!(sidlen > 0, take!(sidlen as usize)) >>
        ciphers:   length_bytes!(be_u16) >>
        comp_len:  take!(1) >>
        comp:      take!(comp_len[0] as usize) >>
        // maybe extensions are optional but they are needed in the use case
        exts_len:   be_u16 >>
        exts:       flat_map!(take!(exts_len), many0!(complete!(parse_tls_extension ))) >>
        (TlsClientHello::new(v,rand_time,rand_data,sid,ciphers,comp, exts))
    )
);

/// Parse one packet only, as plaintext
/// A single record can contain multiple messages, they must share the same record type
#[allow(dead_code)]
named!(parse_tls_plaintext<TlsClientHello>,
    do_parse!(
        tag!( &[ TLS_RECORD_TYPE_HANDSHAKE ][..] ) >>
        _rec_ver: be_u16 >>
        _rec_len: be_u16 >>
        // just try to parse one message
        tag!( &[ TLS_HANDSHAKE_TYPE_CLIENT_HELLO ][..] ) >>
        msg_len: be_u24 >>
        m: flat_map!(take!(msg_len), call!(parse_tls_handshake_msg_client_hello)) >>
        ( m )
    )
);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_tls_record_clienthello() {
        let empty = &b""[..];
        let bytes = [
            0x16, // handshake
            0x03, 0x01,
            0x01, 0x3e, // length: 318 bytes
            0x01, // client hello
            0x00, 0x01, 0x3a, // len: 314
            0x03, 0x03, // tls 1.2
            // random
            0x97, 0x7e, 0xaa, 0x9c, 0x0f, 0xa9, 0xc4, 0x9f,
            0x79, 0x5d, 0xe9, 0x48, 0xa8, 0x26, 0xf0, 0x4a,
            0x93, 0x58, 0x1c, 0x31, 0x00, 0x00, 0x00, 0x00,
            0xa2, 0xb7, 0x11, 0xba, 0x37, 0x05, 0x36, 0x90,

            0x00, // Session ID Length
            0x00, 0xaa, // Cipher Suites Length: 170
            // cipher
            0xca, 0xa0, 0x12, 0x0c, 0xfc, 0x5c, 0x8f, 0xd6, 0x62, 0x92,
            0xd2, 0x2f, 0xa0, 0x1e, 0xeb, 0x59, 0xeb, 0x6e, 0x55, 0x1c,
            0x66, 0x93, 0xde, 0xab, 0x2f, 0x63, 0x75, 0x8a, 0x32, 0x72,
            0x08, 0xb1, 0xf8, 0x6c, 0x92, 0xa7, 0x72, 0x81, 0x9c, 0x33,
            0xd4, 0xf5, 0xbc, 0x06, 0x15, 0xdb, 0xcf, 0x06, 0x28, 0x7c,
            0xce, 0xe8, 0xa6, 0x9f, 0x68, 0x44, 0x1e, 0x95, 0xdf, 0x21,
            0xf5, 0x4a, 0x63, 0x9b, 0xd4, 0x3d, 0xf9, 0x02, 0xfb, 0x4d,
            0x7a, 0x58, 0xf7, 0xf2, 0x20, 0x31, 0x96, 0xc8, 0xf8, 0x1a,
            0xaa, 0x61, 0x06, 0x5f, 0xa7, 0x02, 0xab, 0x86, 0xb8, 0x75,
            0x7c, 0xc0, 0x83, 0x4c, 0x75, 0x2e, 0xa2, 0x48, 0x16, 0x7c,
            0x3a, 0x21, 0x13, 0x0a, 0xd9, 0xf2, 0xf7, 0x38, 0xd2, 0xbf,
            0x0e, 0xec, 0xec, 0xab, 0xdb, 0xd4, 0xdd, 0x14, 0x6b, 0x7c,
            0xeb, 0x8d, 0x2d, 0x60, 0xb9, 0x96, 0xf5, 0x13, 0x5b, 0xf8,
            0xb8, 0x43, 0xa8, 0x44, 0x6a, 0x9d, 0xb2, 0xdd, 0xfe, 0x01,
            0x63, 0x15, 0x1d, 0x07, 0xf7, 0x54, 0x85, 0x7f, 0x77, 0x90,
            0x07, 0x03, 0xc4, 0x24, 0x42, 0x8a, 0xc4, 0xd1, 0x26, 0xed,
            0x03, 0x56, 0x83, 0xd9, 0x9e, 0x9e, 0x1c, 0x7a, 0x9e, 0x78,

            0x01, 0x00, // compression

            0x00, 0x67, // ext len: 103

            0x00, 0x00, // Extension Type: Server Name (check extension type)
            0x00, 0x0e, // Length (use for bounds checking)
            0x00, 0x0c, // Server Name Indication Length
            0x00, // Server Name Type: host_name (check server name type)
            0x00, 0x09, // Length (length of your data)
            // "localhost" (data your after)
            0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x68, 0x6f, 0x73, 0x74,

            0x00, 0x0b, // ec_point_formats
            0x00, 0x04,
            0x03, 0x00, 0x01, 0x02,

            0x00, 0x0a, // supported_groups (renamed from "elliptic_curves")
            0x00, 0x1c, // len: 28
            0x2e, 0x79, 0x60, 0x6c, 0x1e, 0x66, 0xe7, 0x96, 0x7a, 0xa9,
            0x8c, 0xdf, 0x5f, 0xd8, 0x75, 0x91, 0x66, 0x6a, 0xcb, 0x73,
            0x2d, 0x92, 0xea, 0xf8, 0xd8, 0x1d, 0xf7, 0xf5,

            0x00, 0x23, //  	session_ticket (renamed from "SessionTicket TLS")
            0x00, 0x00,

            0x00, 0x0d,  // Signature Algorithms
            0x00, 0x20,
            0xa8, 0x26, 0xf0, 0x4a, 0x93, 0x58, 0x1c, 0x31,
            0xf8, 0x6c, 0x92, 0xa7, 0x72, 0x81, 0x9c, 0x33,
            0x83, 0x4c, 0x75, 0x2e, 0xa2, 0x48, 0x16, 0x7c,
            0xc4, 0x24, 0x42, 0x8a, 0xc4, 0xd1, 0x26, 0xed,

            0x00, 0x0f, // heartbeat
            0x00, 0x01,
            0x01
        ];
        let rand_data = &bytes[15..43];
        let ciphers = &bytes[46..(46+170)];
        let expected = TlsClientHello {
            version: TlsVersion::Tls12,
            rand_time: 0x977eaa9c,
            rand_data: &rand_data,
            session_id: None,
            ciphers: ciphers,
            comp: &[0],
            ext: vec![ TlsExtension::SNI(vec![(0, "localhost".into())]),
                       TlsExtension::Unknown(11, &[3, 0, 1, 2]),
                       TlsExtension::Unknown(10, &[0x2e, 0x79, 0x60, 0x6c, 0x1e, 0x66, 0xe7, 0x96, 0x7a, 0xa9,
            0x8c, 0xdf, 0x5f, 0xd8, 0x75, 0x91, 0x66, 0x6a, 0xcb, 0x73,
            0x2d, 0x92, 0xea, 0xf8, 0xd8, 0x1d, 0xf7, 0xf5,]),
                       TlsExtension::Unknown(35, &[]),
                       TlsExtension::Unknown(13, &[0xa8, 0x26, 0xf0, 0x4a, 0x93, 0x58, 0x1c, 0x31,
            0xf8, 0x6c, 0x92, 0xa7, 0x72, 0x81, 0x9c, 0x33,
            0x83, 0x4c, 0x75, 0x2e, 0xa2, 0x48, 0x16, 0x7c,
            0xc4, 0x24, 0x42, 0x8a, 0xc4, 0xd1, 0x26, 0xed,
                                             ]),
                       TlsExtension::Unknown(15, &[1]),
            ],
        };
        let res = parse_tls_plaintext(&bytes);
        assert_eq!(res, Ok((empty, expected)));
    }
}
