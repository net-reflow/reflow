extern crate reflow;

use std::process;

use reflow::run;

fn main() {
    if let Err(e) = run() {
        println!("error: {:?}", e);
        process::exit(1);
    };
}
