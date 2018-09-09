use tokio_service::Service;
use futures::{future, Future};

use std::io;

use super::ruling;


pub struct Echo {
    ruler: ruling::Ruler
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
        let d = self.ruler.rule_domain(&req);
        if let Some(s) = d {
            Box::new(future::ok(s.to_string()))
        } else {
            Box::new(future::ok("unknown".to_string()))
        }
    }
}

impl Echo {
    pub fn new(config: &str) -> Echo {
        let ruler = ruling::Ruler::new(config);
        Echo {
            ruler: ruler
        }
    }
}
