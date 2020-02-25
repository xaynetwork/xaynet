#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate serde;

use std::env;

use bytes::Bytes;
use config::{Config, ConfigError};
use futures::future::{ready, Ready};

use clap::{App, Arg};

use xain_fl::aggregator::service::{Aggregator, AggregatorService};

#[tokio::main]
async fn main() {
    let matches = App::new("aggregator")
        .version("0.0.1")
        .about("XAIN FL aggregator service")
        .arg(
            Arg::with_name("config")
                .short("c")
                .takes_value(true)
                .help("path to the config file"),
        )
        .get_matches();
    let config_file = matches.value_of("config").unwrap();
    let settings = Settings::new(config_file).unwrap();
    env::set_var("RUST_LOG", &settings.log_level);
    env_logger::init();

    _main(settings).await;
}

async fn _main(settings: Settings) {
    let aggregator = AggregatorService::new(
        DummyAggregator,
        settings.rpc.bind_address,
        settings.rpc.coordinator_address,
    );
    aggregator.await;
}

struct DummyAggregator;

#[async_trait]
impl Aggregator for DummyAggregator {
    type Error = ::std::io::Error;
    type AggregateFut = Ready<Result<Bytes, Self::Error>>;

    async fn add_weights(&mut self, _weights: Bytes) -> Result<(), Self::Error> {
        ready(Ok(())).await
    }
    fn aggregate(&mut self) -> Self::AggregateFut {
        ready(Ok(Bytes::new()))
    }
}

#[derive(Debug, Deserialize)]
struct Settings {
    log_level: String,
    api: ApiSettings,
    rpc: RpcSettings,
}

#[derive(Debug, Deserialize)]
struct ApiSettings {
    bind_address: String,
}

#[derive(Debug, Deserialize)]
struct RpcSettings {
    bind_address: String,
    coordinator_address: String,
}

impl Settings {
    pub fn new(path: &str) -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(config::File::with_name(path)).unwrap();
        s.try_into()
    }
}
