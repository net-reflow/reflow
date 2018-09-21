use bytes::BytesMut;
use httparse;

mod tls;

use self::tls::TlsClientHello;
use std::borrow::Cow;

#[derive(Debug)]
pub enum TcpTrafficInfo<'a> {
    PlainHttp(HttpInfo<'a>),
    SSH,
    Tls(TlsClientHello<'a>),
    Unidentified,
}

#[derive(Debug)]
pub struct HttpInfo<'a> {
    host: Cow<'a, str>,
    user_agent: Option<Cow<'a, str>>,
}

impl<'a> HttpInfo<'a> {
    pub fn new(h: &'a[u8], ua: Option<&'a[u8]>)-> HttpInfo<'a> {
        HttpInfo {
            host: String::from_utf8_lossy(h),
            user_agent: ua.map(|b| String::from_utf8_lossy(b)),
        }
    }
}

pub fn route(bytes: &BytesMut)-> Option<String> {
    guess_bytes(bytes).map(|k| format!("{:?}", k))
}

fn guess_bytes(bytes: &BytesMut) ->Option<TcpTrafficInfo> {
    debug!("bytes {:?}", bytes);
    if let Some(h) = guess_http(bytes) { return Some(TcpTrafficInfo::PlainHttp(h)) }
    if bytes.starts_with(b"SSH-2.0") && bytes.ends_with(b"\r\n") {
        return Some(TcpTrafficInfo::SSH);
    }
    return Some(TcpTrafficInfo::Unidentified);
}

fn guess_http(bytes: &BytesMut)->Option<HttpInfo> {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);
    let status = req.parse(bytes).ok()?;
    let mut host = None;
    let mut ua = None;
    for header in req.headers {
        if ascii_stc_eq_ignore_case(header.name, "Host") {
            host = Some(header.value);
        } else if ascii_stc_eq_ignore_case(header.name, "User-Agent") {
            ua = Some(header.value);
        }
    }
    match host {
        None => None,
        Some(h) => {
            if ua.is_some() || status.is_complete() {
                Some(HttpInfo::new(h, ua))
            } else { None }
        }
    }
}

fn ascii_stc_eq_ignore_case(a: &str, b: &str)-> bool {
    a.len() == b.len() &&
        a.as_bytes().iter().zip(b.as_bytes().iter())
            .all(|(ac, bc)| ac.eq_ignore_ascii_case(bc))
}