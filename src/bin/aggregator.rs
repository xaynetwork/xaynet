#[macro_use]
extern crate async_trait;

use std::{env, process};

use bytes::Bytes;
use futures::future::{ready, Ready};

use clap::{App, Arg};

use xain_fl::aggregator::{
    api,
    service::{Aggregator, AggregatorService},
    settings::Settings,
};

#[tokio::main]
async fn main() {
    let matches = App::new("aggregator")
        .version("0.0.1")
        .about("XAIN FL aggregator service")
        .arg(
            Arg::with_name("config")
                .short("c")
                .takes_value(true)
                .required(true)
                .help("path to the config file"),
        )
        .get_matches();
    let config_file = matches.value_of("config").unwrap();

    let settings = Settings::new(config_file).unwrap_or_else(|err| {
        eprintln!("Problem parsing configuration file: {}", err);
        process::exit(1);
    });
    env::set_var("RUST_LOG", &settings.log_level);
    env_logger::init();

    _main(settings).await;
}

async fn _main(settings: Settings) {
    let Settings { rpc, api, .. } = settings;

    let (aggregator, handle) =
        AggregatorService::new(DummyAggregator, rpc.bind_address, rpc.coordinator_address);

    tokio::spawn(async move { api::serve(&api.bind_address, handle).await });
    aggregator.await;
}

struct DummyAggregator;

impl Aggregator for DummyAggregator {
    type Error = ::std::io::Error;
    type AggregateFut = Ready<Result<Bytes, Self::Error>>;
    type AddWeightsFut = Ready<Result<(), Self::Error>>;

    fn add_weights(&mut self, _weights: Bytes) -> Self::AddWeightsFut {
        ready(Ok(()))
    }
    fn aggregate(&mut self) -> Self::AggregateFut {
        ready(Ok(Bytes::new()))
    }
}
