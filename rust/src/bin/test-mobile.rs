#[macro_use]
extern crate tracing;

use std::io::{stdin, stdout, Read, Write};
use structopt::StructOpt;
use tracing_subscriber::*;
use xain_fl::{
    client::mobile_client::MobileClient,
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

fn pause() {
    let mut stdout = stdout();
    stdout.write(b"Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    stdin().read(&mut [0]).unwrap();
}

fn main() -> Result<(), ()> {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    let mut client = MobileClient::new("http://localhost:8081");

    let model = Model::from_primitives(vec![1; 4].into_iter()).unwrap();

    loop {
        client.set_local_model(model.clone());
        client.next();
        pause();
    }
}
