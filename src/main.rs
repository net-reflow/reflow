extern crate reflow;

use std::process;

use reflow::run;

fn main() {
    println!("Hello, world!");
    if let Err(e) = run() {
        println!("error: {:?}", e);
        process::exit(1);
    };
}
