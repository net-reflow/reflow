extern crate droxy;

use std::env;

use droxy::run;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config_path: &str = if args.len() == 1 {
        "config"
    } else {
        let option = args.get(1).unwrap();
        if !(option == "-c" || option == "--config") {
            panic!("option {} not recognized", option);
        }
        args.get(2).expect("missing configuration path")
    };
    println!("Hello, world!");
    if let Err(e) = run(&config_path) {
        println!("error: {:?}", e);
    };
}
