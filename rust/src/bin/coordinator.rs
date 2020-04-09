#[macro_use]
extern crate tracing;

use clap::{App, Arg};
use rand::seq::IteratorRandom;
use std::{path::Path, process};
use tokio::signal::ctrl_c;
use tracing_futures::Instrument;

#[cfg(feature = "influx_metrics")]
use xain_fl::{
    common::metric_store::influxdb::{run_metricstore, InfluxDBConnector},
    coordinator::settings::MetricStoreSettings,
};

use xain_fl::{
    aggregator,
    common::{client::ClientId, logging},
    coordinator::{
        api,
        core::{Selector, Service, ServiceHandle},
        rpc,
        settings::{ApiSettings, FederatedLearningSettings, RpcSettings, Settings},
    },
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

    #[rustfmt::skip]
    let Settings {
        rpc,
        api,
        federated_learning,
        aggregator_url,
        #[cfg(feature = "influx_metrics")]
        // FIXME: when compiling without the `influx_metrics` feature,
        // rustc emits a warning about this variable being
        // unused. That looks like a bug, so we silence rustc by
        // prefixing the variable name with an underscore.
        //
        // Also, rustfmt doesn't really like this syntax, so we
        // disable it here
        metric_store: _metric_store,
        logging,
        ..
    } = settings;
    logging::configure(logging);

    let span = trace_span!("root");
    _main(
        rpc,
        api,
        federated_learning,
        aggregator_url,
        #[cfg(feature = "influx_metrics")]
        _metric_store,
    )
    .instrument(span)
    .await;
}

async fn _main(
    rpc: RpcSettings,
    api: ApiSettings,
    federated_learning: FederatedLearningSettings,
    aggregator_url: String,
    #[cfg(feature = "influx_metrics")] metric_store: Option<MetricStoreSettings>,
) {
    let exit_file_path = "./exit_state.lock";

    let exit_state = get_last_exit_state(exit_file_path)
        .await
        .expect("Error reading last exit state:");

    match exit_state {
        ExitState::Dirty(r) => warn!("Dirty exit on round {}. Try to sync with aggregator.", r),
        ExitState::Clean => info!("Clean start"),
        // start rpc()
        // How do we update the round number? using channels?
    }

    let (service_handle, service_requests) = ServiceHandle::new();

    // Start the RPC server
    let rpc_server = rpc::serve(rpc.bind_address.clone(), service_handle.clone())
        .instrument(trace_span!("rpc_server"));
    let rpc_server_task_handle = tokio::spawn(rpc_server);

    // Start the RPC client
    let rpc_client = tokio::select! {
        client = aggregator::rpc::Client::connect(rpc.aggregator_address.clone()).instrument(trace_span!("rpc_client")) => {client.unwrap()}
        result = ctrl_c() => {
            match result {
                Ok(()) => {info!("shutting down: received SIGINT");
                mark_clean_exit(exit_file_path).await; // on clean exit, remove file
            },
                Err(e) => error!("shutting down: error while waiting for SIGINT: {}", e),
            }
            return
        }
    };

    #[cfg(feature = "influx_metrics")]
    let metric_sender = if let Some(metric_store) = metric_store {
        // Start the metric store
        let (influx_client, metric_sender) = InfluxDBConnector::new(
            &metric_store.database_url[..],
            &metric_store.database_name[..],
        );

        let _ = tokio::spawn(async move { run_metricstore(influx_client).await });
        Some(metric_sender)
    } else {
        None
    };

    // Start the api server
    let api_server_task_handle = tokio::spawn(
        async move { api::serve(api.bind_address.as_str(), service_handle.clone()).await }
            .instrument(trace_span!("api_server")),
    );

    // Create the service
    let service = Service::new(
        RandomSelector,
        federated_learning,
        aggregator_url,
        rpc_client,
        service_requests,
        #[cfg(feature = "influx_metrics")]
        metric_sender,
    );

    // Run the service, and wait for one of the tasks to terminate
    tokio::select! {
        _ = service.instrument(trace_span!("service")) => {
            info!("shutting down: CoordinatorService terminated");
        }
        _ = api_server_task_handle => {
            info!("shutting down: API task terminated");
        }
        _ = rpc_server_task_handle => {
            info!("shutting down: RPC server task terminated");
        }
        result = ctrl_c() => {
            match result {
                Ok(()) => {info!("shutting down: received SIGINT");
                mark_clean_exit(exit_file_path).await; // on clean exit, remove file
            },
                Err(e) => error!("shutting down: error while waiting for SIGINT: {}", e),

            }
        }
    }
}

enum ExitState {
    Clean,
    Dirty(u32),
}

async fn get_last_exit_state(
    path: &str,
) -> Result<ExitState, Box<dyn std::error::Error + 'static>> {
    let path = Path::new(path);

    if path.exists() {
        // the coordinator exited dirty
        let contents = tokio::fs::read(path).await?;
        let last_round_number = String::from_utf8_lossy(&contents);
        if last_round_number.is_empty() {
            // the coordinator exited on round zero
            return Ok(ExitState::Dirty(0));
        }

        Ok(ExitState::Dirty(last_round_number.parse()?))
    } else {
        // the coordinator exited clean
        tokio::fs::File::create(path).await?;
        Ok(ExitState::Clean)
    }
}

async fn mark_clean_exit(path: &str) {
    let path = Path::new(path);
    tokio::fs::remove_file(path)
        .await
        .expect("Cannot remove exit_state file:");
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
