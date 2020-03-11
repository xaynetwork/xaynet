#[macro_use]
extern crate log;

use clap::{App, Arg};
use rand::seq::IteratorRandom;
use std::{env, process};
use tokio::sync::mpsc;

use xain_fl::{
    aggregator,
    common::ClientId,
    coordinator::{
        api,
        core::{CoordinatorService, Selector},
        rpc,
        settings::Settings,
    },
    metric_store::influxdb::{run_metricstore, InfluxDBMetricStore},
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

    let (rpc_request_stream_tx, rpc_request_stream_rx) = mpsc::channel(1);
    // It is important to start the RPC server before starting an RPC
    // client, because if both the aggregator and the coordinator
    // attempt to connect to each other before the servers are
    // started, we end up in a deadlock.
    let rpc_server_task_handle =
        tokio::spawn(rpc::serve(rpc.bind_address.clone(), rpc_request_stream_tx));
    let rpc_requests = rpc::RpcRequestsMux::new(rpc_request_stream_rx);
    let rpc_client = aggregator::rpc::client_connect(rpc.aggregator_address.clone())
        .await
        .unwrap();

    let (influx_client, metric_sender) = InfluxDBMetricStore::new(
        &metric_store.database_url[..],
        &metric_store.database_name[..],
    );

    let _ = tokio::spawn(async move { run_metricstore(influx_client).await });

    let (service, handle) = CoordinatorService::new(
        RandomSelector,
        federated_learning,
        aggregator_url,
        rpc_client,
        rpc_requests,
        metric_sender,
    );

    let api_task_handle = tokio::spawn(async move { api::serve(&api.bind_address, handle).await });

    tokio::select! {
        _ = service => {
            info!("shutting down: CoordinatorService terminated");
        }
        _ = api_task_handle => {
            info!("shutting down: API task terminated");
        }
        _ = rpc_server_task_handle => {
            info!("shutting down: RPC server task terminated");
        }
    }
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
