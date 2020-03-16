use clap::{App, Arg};
use std::{env, process};
use tokio::{signal::ctrl_c, sync::mpsc};
use tracing_subscriber::{filter::EnvFilter, fmt::time::ChronoUtc, FmtSubscriber};
use xain_fl::{
    aggregator::{
        api,
        py_aggregator::spawn_py_aggregator,
        rpc,
        service::AggregatorService,
        settings::{AggregationSettings, Settings},
    },
    coordinator,
};
#[macro_use]
extern crate tracing;

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

    configure_tracing();

    _main(settings).await;
}

async fn _main(settings: Settings) {
    let Settings {
        rpc,
        api,
        aggregation,
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

    let rpc_client = coordinator::rpc::client_connect(rpc.coordinator_address.clone())
        .await
        .unwrap();

    let (aggregator, mut shutdown_rx) = match aggregation {
        AggregationSettings::Python(python_aggregator_settings) => {
            spawn_py_aggregator(python_aggregator_settings)
        }
    };

    // Spawn the task that waits for the aggregator running in a
    // background thread to finish.
    let aggregator_task_handle = tokio::spawn(async move { shutdown_rx.recv().await });

    let (service, handle) = AggregatorService::new(aggregator, rpc_client, rpc_requests);

    // Spawn the task that provides the public HTTP API.
    let api_task_handle = tokio::spawn(async move { api::serve(&api.bind_address, handle).await });

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
        _ = rpc_server_task_handle => {
            info!("shutting down: RPC server task terminated");
        }
        result = ctrl_c() => {
            match result {
                Ok(()) => info!("shutting down: received SIGINT"),
                Err(e) => error!("shutting down: error while waiting for SIGINT: {}", e),

            }
        }
    }
}

fn configure_tracing() {
    let subscriber = FmtSubscriber::builder()
        .with_ansi(true)
        .with_timer(ChronoUtc::rfc3339())
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("failed to setup tracing");
}
