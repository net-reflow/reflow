use std::path::PathBuf;

#[derive(StructOpt)]
pub struct Opt {
    #[structopt(short = "c", long = "config")]
    /// The directory of configuration
    #[structopt(parse(from_os_str))]
    pub config: PathBuf,
}
