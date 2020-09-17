use std::{path::PathBuf, process};
use structopt::StructOpt;
use tokio::signal;
use tracing_subscriber::*;
use xaynet_server::{
    rest,
    services,
    settings::Settings,
    state_machine::StateMachine,
    storage::redis,
};

#[cfg(feature = "metrics")]
use xaynet_server::metrics::{run_metric_service, MetricsService};

#[macro_use]
extern crate tracing;

#[derive(Debug, StructOpt)]
#[structopt(name = "Coordinator")]
struct Opt {
    /// Path of the configuration file
    #[structopt(short, parse(from_os_str))]
    config_path: PathBuf,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    #[cfg_attr(not(feature = "metrics"), allow(unused_variables))]
    let Settings {
        pet: pet_settings,
        mask: mask_settings,
        api: api_settings,
        log: log_settings,
        model: model_settings,
        metrics: metrics_settings,
        redis: redis_settings,
    } = Settings::new(opt.config_path).unwrap_or_else(|err| {
        eprintln!("{}", err);
        process::exit(1);
    });

    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(log_settings.filter)
        .with_ansi(true)
        .init();

    // This should already called internally when instantiating the
    // state machine but it doesn't hurt making sure the crypto layer
    // is correctly initialized
    sodiumoxide::init().unwrap();

    #[cfg(feature = "metrics")]
    let (metrics_sender, metrics_handle) = {
        let (metrics_service, metrics_sender) = MetricsService::new(
            &metrics_settings.influxdb.url,
            &metrics_settings.influxdb.db,
        );
        (
            metrics_sender,
            tokio::spawn(async { run_metric_service(metrics_service).await }),
        )
    };

    let redis = redis::Client::new(redis_settings.url, 100)
        .await
        .expect("failed to establish a connection to Redis");
    redis
        .connection()
        .await
        .flush_db()
        .await
        .expect("failed to flush the Redis database");

    let (state_machine, requests_tx, event_subscriber) = StateMachine::new(
        pet_settings,
        mask_settings,
        model_settings,
        redis,
        #[cfg(feature = "metrics")]
        metrics_sender,
    )
    .unwrap();
    let fetcher = services::fetchers::fetcher(&event_subscriber);
    let message_handler =
        services::messages::PetMessageHandler::new(&event_subscriber, requests_tx);

    tokio::select! {
        _ = state_machine.run() => {
            warn!("shutting down: Service terminated");
        }
        _ = rest::serve(api_settings.bind_address, fetcher, message_handler) => {
            warn!("shutting down: REST server terminated");
        }
        _ =  signal::ctrl_c() => {}
    }

    #[cfg(feature = "metrics")]
    {
        // The moment the state machine is dropped, the sender half of the metrics channel is also
        // dropped, which means that the metric handle is resolved after all remaining messages have
        // been processed.
        warn!("shutting down metrics service");
        let _ = metrics_handle.await;
    }
}
