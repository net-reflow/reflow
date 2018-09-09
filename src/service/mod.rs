use tokio_service::Service;
use futures::{future, Future};

use std::io;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::Arc;

use super::ruling;


pub struct Echo {
    ruler: Arc<ruling::Ruler>
}

impl Service for Echo {
    // These types must match the corresponding protocol types:
    type Request = String;
    type Response = String;

    // For non-streaming protocols, service errors are always io::Error
    type Error = io::Error;

    // The future for computing the response; box it for simplicity.
    type Future = Box<Future<Item = Self::Response, Error =  Self::Error>>;

    // Produce a future for computing a response from a request.
    fn call(&self, req: Self::Request) -> Self::Future {
        // In this case, the response is immediate.
        let r = self.process(&req);
        Box::new(future::ok(r))
    }
}

impl Echo {
    pub fn new(ruler: Arc<ruling::Ruler>) -> Echo {
        Echo {
            ruler: ruler
        }
    }

    fn process(&self, req: &str) -> String {
        let ws: Vec<&str> = req.splitn(2, ' ').collect();
        if ws.len() < 2 {
            return "error too few args".to_string()
        }
        let t = ws[0];
        let a = ws[1];
        if t == "d" {
            let d = self.ruler.rule_domain(a);
            if let Some(s) = d {
                s.to_string()
            } else {
                "none".to_string()
            }
        } else if t == "i4" {
            let ip = Ipv4Addr::from_str(a);
            if let Ok(ip) = ip {
                if let Some(r) = self.ruler.rule_ip4(ip) {
                    r.to_string()
                } else {
                    "none".to_string()
                }
            } else {
                "error parsing ip".to_string()
            }
        } else {
            "error unknown type".to_string()
        }
    }
}
