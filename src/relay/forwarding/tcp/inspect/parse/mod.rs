use bytes::BytesMut;
use bytes::Bytes;
use httparse;

mod tls;

use self::tls::{TlsWithSni, parse_tls_sni};

#[derive(Debug)]
pub enum TcpProtocol {
    PlainHttp(HttpInfo),
    SSH,
    Tls(TlsWithSni),
    Unidentified,
}

impl TcpProtocol {
    /// for matching keys in a dictionary
    pub fn name(&self) -> &[u8] {
        use self::TcpProtocol::*;
        match &self {
            PlainHttp(_) => b"http",
            SSH => b"ssh",
            Tls(_) => b"tls",
            Unidentified => b"unidentified",
        }
    }

    pub fn get_domain(&self)-> Option<&[u8]> {
        use self::TcpProtocol::*;
        match &self {
            PlainHttp(x) => Some(&x.host),
            Tls(x) => Some(&x.sni),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct HttpInfo {
    host: Bytes,
    user_agent: Option<Bytes>,
}

impl HttpInfo {
    pub fn new(h: &[u8], ua: Option<&[u8]>)-> HttpInfo {

        HttpInfo {
            host: BytesMut::from( h).freeze(),
            user_agent: ua.map(|b| BytesMut::from(b).freeze()),
        }
    }
}

pub fn guess_bytes(bytes: &BytesMut) ->TcpProtocol {
    trace!("bytes {:?}", bytes);
    if let Some(h) = guess_http(bytes) { return TcpProtocol::PlainHttp(h) }
    if bytes.starts_with(b"SSH-2.0") {
        return TcpProtocol::SSH;
    }
    if let Some(x) = parse_tls_sni(bytes.as_ref()) { return TcpProtocol::Tls(x) }
    return TcpProtocol::Unidentified;
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