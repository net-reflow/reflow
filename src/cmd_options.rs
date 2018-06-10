use std::path::PathBuf;

#[derive(StructOpt)]
#[structopt(name = "peroxide")]
pub struct Opt {
    #[structopt(short = "c", long = "config",
    default_value = "./config")]
    /// The directory of configuration
    #[structopt(parse(from_os_str))]
    pub config: PathBuf,
}
