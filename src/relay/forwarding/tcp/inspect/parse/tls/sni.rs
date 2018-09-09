use nom::{be_u8,be_u16, IResult};

/// TLS extensions
///
#[derive(Clone, Debug, PartialEq)]
pub enum TlsExtension<'a>{
    SNI(Vec<(u8,&'a[u8])>),
    Unknown(u16,&'a[u8]),
}

named!(pub parse_tls_extension_sni_hostname<(u8,&[u8])>,
    pair!(be_u8,length_bytes!(be_u16))
);

named!(pub parse_tls_extension_sni_content<TlsExtension>,
    do_parse!(
        list_len: be_u16 >>
        v: flat_map!(take!(list_len),
            many0!(complete!(parse_tls_extension_sni_hostname))
        ) >>
        ( TlsExtension::SNI(v) )
    )
);

named!(pub parse_tls_extension_sni<TlsExtension>,
    do_parse!(
        tag!([0x00,0x00]) >>
        ext_len:  be_u16 >>
        ext: flat_map!(take!(ext_len),parse_tls_extension_sni_content) >>
        ( ext )
    )
);

fn parse_tls_extension_with_type(i: &[u8], ext_type:u16, ext_len:u16) -> IResult<&[u8],TlsExtension> {
    match ext_type {
        0x0000 => parse_tls_extension_sni_content(i),
        _      => { map!(i, take!(ext_len), |ext_data| { TlsExtension::Unknown(ext_type,ext_data) }) },
    }
}
named!(pub parse_tls_extension<TlsExtension>,
   do_parse!(
       ext_type: be_u16 >>
       ext_len:  be_u16 >>
       ext: flat_map!(take!(ext_len), call!(parse_tls_extension_with_type, ext_type, ext_len)) >>
       ( ext )
   )
);