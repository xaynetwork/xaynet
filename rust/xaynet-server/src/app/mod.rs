use crate::rest::RestError;
use crate::settings::Settings;

use futures::Future;
use structopt::StructOpt;
use thiserror::Error;
use tracing::warn;

pub mod admin;
pub mod components;
pub mod drain;
pub mod signal;

use components::api;
use components::config;
use components::crypto;
use components::metrics;
use components::state_machine;
use components::store;
use components::terminal;
use components::trazing;
use trazing::Tracing;

use self::components::terminal::Opt;

/// Error that occurs during the update phase.
#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("initializing crypto failed.")]
    Crypto,
    #[error("initializing crypto failed.")]
    Config,
    #[error("initializing crypto failed.")]
    Metrics,
    #[error("initializing crypto failed.")]
    Trace,
    #[error("initializing crypto failed.")]
    StateMachine,
}

pub async fn bootstrap() {
    let (drain_tx, drain_rx) = drain::channel();
    let ready = Readiness::new();

    tokio::select! {
        _ =  run_app(drain_rx, ready) => {
            warn!("");
        }
        _ = signal::shutdown() => {
            warn!("graceful shutdown");
        }
    }

    drain_tx.drain().await;
}

async fn run_app(shutdown: drain::Watch, ready: Readiness) -> Result<(), ApplicationError> {
    let trace = trazing::Tracing::default();
    let admin_api = admin::build(shutdown.clone(), ready.clone(), trace.clone());
    tokio::spawn(admin_api);

    let opt = terminal::Opt::from_args();
    let (state_machine, api) = init_components(opt, &trace, shutdown, ready).await?;
    tokio::spawn(api);
    tokio::spawn(state_machine);

    futures::future::pending::<Result<(), ApplicationError>>().await
}

async fn init_components(
    opt: Opt,
    trace: &Tracing,
    shutdown: drain::Watch,
    ready: Readiness,
) -> Result<
    (
        impl Future<Output = ()> + 'static,
        impl Future<Output = Result<(), RestError>> + 'static,
    ),
    ApplicationError,
> {
    crypto::init().map_err(|_| ApplicationError::Crypto)?;

    let settings = config::init(&opt.config_path).map_err(|_| ApplicationError::Config)?;
    let Settings {
        pet: pet_settings,
        mask: mask_settings,
        api: api_settings,
        model: model_settings,
        log: log_settings,
        redis: redis_settings,
        metrics: metrics_settings,
        ..
    } = settings;

    trace
        .reload(log_settings.filter)
        .map_err(|_| ApplicationError::Trace)?;

    metrics::init(metrics_settings).map_err(|_| ApplicationError::Metrics)?;

    let store = store::init(
        redis_settings,
        #[cfg(feature = "model-persistence")]
        settings.s3,
    )
    .await;

    let (state_machine, requests_tx, event_subscriber) = state_machine::init(
        pet_settings,
        mask_settings,
        model_settings,
        #[cfg(feature = "model-persistence")]
        restore_settings,
        store,
    )
    .await
    .map_err(|_| ApplicationError::StateMachine)?;

    ready.inject(&event_subscriber);

    let api = api::init(
        api_settings,
        event_subscriber,
        requests_tx,
        shutdown.clone(),
    );

    Ok((state_machine.run(shutdown), api))
}

use crate::state_machine::events::EventListener;
use crate::state_machine::events::EventSubscriber;
use arc_swap::ArcSwapOption;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Readiness(Arc<ArcSwapOption<EventListener<bool>>>);

impl Readiness {
    pub fn new() -> Self {
        Self(Arc::new(ArcSwapOption::from(None)))
    }

    pub fn inject(&self, event_sub: &EventSubscriber) {
        self.0.swap(Some(Arc::new(event_sub.readiness_listener())));
    }

    pub fn is_ready(&self) -> bool {
        match self.0.load_full() {
            None => false,
            Some(sub) => sub.get_latest().event,
        }
    }
}
