extern crate droxy;

use std::env;

use droxy::run;

fn main() {
    let mut argv = env::args();
    let port = argv.nth(1).unwrap();
    println!("Hello, world!");
    if let Err(e) = run(&port) {
        println!("error: {:?}", e);
    };
}
