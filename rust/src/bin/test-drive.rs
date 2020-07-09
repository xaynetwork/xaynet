#[macro_use]
extern crate tracing;

use rand::Rng;
use std::thread;
use structopt::StructOpt;
use tokio::{runtime, signal};
use tracing_subscriber::*;
use xain_fl::{
    client::{ClientError, SyncClient},
    mask::{FromPrimitives, Model},
};

#[derive(Debug, StructOpt)]
#[structopt(name = "Test Drive")]
struct Opt {
    #[structopt(
        default_value = "http://127.0.0.1:8081",
        short,
        help = "The URL of the coordinator"
    )]
    url: String,
    #[structopt(default_value = "4", short, help = "The length of the model")]
    len: u32,
    #[structopt(
        default_value = "1",
        short,
        help = "The time period at which to poll for service data, in seconds"
    )]
    period: u64,
    #[structopt(default_value = "10", short, help = "The number of clients")]
    nb_client: u32,
}

fn main() -> Result<(), ClientError> {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let mut rng = rand::thread_rng();
    let opt = Opt::from_args();
    let len = opt.len as usize;

    let mut client = SyncClient::new("http://127.0.0.1:8081");
    client.start();

    thread::sleep(std::time::Duration::from_secs(5));

    error!("{:?}", client.get_global_model());
    client.stop();

    client.start();

    thread::sleep(std::time::Duration::from_secs(5));

    error!("{:?}", client.get_global_model());
    client.stop();
    Ok(())
}
