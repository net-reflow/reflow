extern crate reflow;

use std::process;

use reflow::run;

fn main() {
    println!("Starting reflow");
    if let Err(e) = run() {
        process::exit(e);
    };
}
