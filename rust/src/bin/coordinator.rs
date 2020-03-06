use clap::{App, Arg};
use rand::seq::IteratorRandom;
use std::{env, process};

use xain_fl::{
    common::ClientId,
    coordinator::{
        api,
        core::{CoordinatorService, Selector},
        settings::Settings,
    },
    metric_store::metric_store::{run_metricstore, InfluxDBMetricStore},
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
                .required(true)
                .help("Path to the config file"),
        )
        .get_matches();
    let config_file = matches.value_of("config").unwrap();

    let settings = Settings::new(config_file).unwrap_or_else(|err| {
        eprintln!("Problem parsing configuration file: {}", err);
        process::exit(1);
    });

    env::set_var("RUST_LOG", &settings.log_level);
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_micros().init();

    _main(settings).await;
}

async fn _main(settings: Settings) {
    let Settings {
        rpc,
        api,
        federated_learning,
        aggregator_url,
        metric_store,
        ..
    } = settings;

    let (influx_client, metric_sender) = InfluxDBMetricStore::new(
        &metric_store.database_url[..],
        &metric_store.database_name[..],
    );

    let (coordinator, handle) = CoordinatorService::new(
        RandomSelector,
        federated_learning,
        aggregator_url,
        rpc.bind_address,
        rpc.aggregator_address,
        metric_sender,
    );

    tokio::spawn(async move { api::serve(&api.bind_address, handle).await });

    tokio::spawn(async move { run_metricstore(influx_client).await });

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
