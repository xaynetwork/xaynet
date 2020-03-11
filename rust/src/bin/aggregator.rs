use clap::{App, Arg};
use std::{env, process};
use xain_fl::aggregator::{
    api,
    py_aggregator::spawn_py_aggregator,
    service::AggregatorService,
    settings::{AggregationSettings, Settings},
};
#[macro_use]
extern crate log;

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
    let Settings {
        rpc,
        api,
        aggregation,
        ..
    } = settings;

    let (aggregator, mut shutdown_rx) = match aggregation {
        AggregationSettings::Python(python_aggregator_settings) => {
            spawn_py_aggregator(python_aggregator_settings)
        }
    };

    let (service, handle) =
        AggregatorService::new(aggregator, rpc.bind_address, rpc.coordinator_address);

    // Spawn the task that provides the public HTTP API.
    let api_task_handle = tokio::spawn(async move { api::serve(&api.bind_address, handle).await });
    // Spawn the task that waits for the aggregator running in a
    // background thread to finish.
    let aggregator_task_handle = tokio::spawn(async move { shutdown_rx.recv().await });

    tokio::select! {
        _ = service => {
            info!("shutting down: AggregatorService terminated");
        }
        _ = aggregator_task_handle => {
            info!("shutting down: Aggregator terminated");
        }
        _ = api_task_handle => {
            info!("shutting down: API task terminated");
        }
    }
}
