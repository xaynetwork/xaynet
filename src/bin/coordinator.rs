#[macro_use]
extern crate serde;
use clap::{App, Arg};
use config::{Config, ConfigError};
use rand::seq::IteratorRandom;
use std::env;

use xain_fl::{
    common::ClientId,
    coordinator::core::{CoordinatorConfig, CoordinatorService, Selector},
};

#[tokio::main]
async fn main() {
    let matches = App::new("coordinator")
        .version("0.0.1")
        .about("XAIN FL coordinator service")
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
    let config = CoordinatorConfig {
        rounds: 3,
        min_clients: 3,
        participants_ratio: 0.5,
    };
    let (coordinator, _handle) = CoordinatorService::new(
        RandomSelector,
        config,
        settings.rpc.bind_address,
        settings.rpc.aggregator_address,
    );
    coordinator.await;
}

pub struct RandomSelector;

impl Selector for RandomSelector {
    fn select(
        &mut self,
        min_count: usize,
        waiting: impl Iterator<Item = ClientId>,
        _selected: impl Iterator<Item = ClientId>,
    ) -> Vec<ClientId> {
        waiting.choose_multiple(&mut rand::thread_rng(), min_count)
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
    aggregator_address: String,
}

impl Settings {
    pub fn new(path: &str) -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(config::File::with_name(path)).unwrap();
        s.try_into()
    }
}
