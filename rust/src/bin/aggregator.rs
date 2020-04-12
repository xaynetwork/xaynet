use clap::{App, Arg};
use std::process;
use tokio::signal::ctrl_c;
use tracing_futures::Instrument;
use xain_fl::{
    aggregator::{
        api,
        py_aggregator::spawn_py_aggregator,
        rpc,
        service::{Service, ServiceHandle},
        settings::{AggregationSettings, ApiSettings, RpcSettings, Settings},
    },
    common::{
        logging,
        state::StateHandle,
        sync::{SyncHandle, SyncRequest, SyncService},
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

    let Settings {
        rpc,
        api,
        aggregation,
        logging,
    } = settings;

    logging::configure(logging);

    let span = trace_span!("root");
    _main(rpc, api, aggregation).instrument(span).await;
}

async fn _main(rpc: RpcSettings, api: ApiSettings, aggregation: AggregationSettings) {
    let (service_handle, service_requests) = ServiceHandle::new();

    let (state_handle, _) = StateHandle::new();
    //let state_service = StateService::new("./state.lock", state_requests);
    let (sync_handle, sync_requests) = SyncHandle::new();

    let rpc_server = rpc::serve(
        rpc.bind_address.clone(),
        service_handle.clone(),
        sync_handle.clone(),
    )
    .instrument(trace_span!("rpc_server"));
    let rpc_server_task_handle = tokio::spawn(rpc_server);

    let rpc_client_span = trace_span!("rpc_client");
    let sync_handle_closure = sync_handle.clone();
    let rpc_client = coordinator::rpc::client_connect(rpc.coordinator_address.clone(), move || {
        let sync_handle_closure = sync_handle_closure.clone();
        tokio::spawn(async move {
            let _ = sync_handle_closure
                .sync(SyncRequest::RPCClientDisconnect)
                .await;
        });
    })
    .instrument(rpc_client_span.clone())
    .await
    .unwrap();

    // Start sync handler
    let sync_service = SyncService::new(
        service_handle.clone(),
        rpc_client.clone(),
        state_handle.clone(),
        sync_requests,
    );

    let (aggregator, mut shutdown_rx) = match aggregation {
        AggregationSettings::Python(python_aggregator_settings) => {
            spawn_py_aggregator(python_aggregator_settings)
        }
    };

    // Spawn the task that waits for the aggregator running in a
    // background thread to finish.
    let aggregator_task_handle = tokio::spawn(async move { shutdown_rx.recv().await });

    // Spawn the task that provides the public HTTP API.
    let api_task_handle = tokio::spawn(
        async move { api::serve(&api.bind_address, service_handle.clone()).await }
            .instrument(trace_span!("api_server")),
    );

    let service = Service::new(aggregator, rpc_client, service_requests);

    tokio::select! {
        _ = service.instrument(trace_span!("service")) => {
            info!("shutting down: Service terminated");
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
        _ = sync_service => {
            info!("shutting down: Sync service task terminated");
        }
        result = ctrl_c() => {
            match result {
                Ok(()) => info!("shutting down: received SIGINT"),
                Err(e) => error!("shutting down: error while waiting for SIGINT: {}", e),

            }
        }
    }
}
