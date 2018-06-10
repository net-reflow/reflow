extern crate droxy;

use droxy::run;

fn main() {
    println!("Hello, world!");
    if let Err(e) = run() {
        println!("error: {:?}", e);
    };
}
