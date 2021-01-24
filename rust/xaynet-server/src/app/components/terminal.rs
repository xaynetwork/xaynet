use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "Xaynet Server")]
pub struct Opt {
    /// Path of the configuration file
    #[structopt(short, parse(from_os_str))]
    pub config_path: PathBuf,
}
