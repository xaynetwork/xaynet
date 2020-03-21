use crate::common::settings::LoggingSettings;
use tracing_subscriber::{fmt::time::ChronoUtc, FmtSubscriber};

#[cfg(not(feature = "telemetry"))]
pub fn configure(settings: LoggingSettings) {
    let fmt_subscriber = FmtSubscriber::builder()
        .with_ansi(true)
        .with_timer(ChronoUtc::rfc3339())
        .with_env_filter(settings.filter)
        .finish();
    // Set the previously created subscriber as the global subscriber
    tracing::subscriber::set_global_default(fmt_subscriber).expect("failed to setup tracing");
    // Redirect normal log messages to the tracing subscriber
    tracing_log::LogTracer::init().unwrap();
    if settings.telemetry.is_some() {
        warn!("ignoring `logging.telemetry` configuration: this binary has been compiled without the telemetry feature");
    }
}

#[cfg(feature = "telemetry")]
mod telemetry {
    use super::*;
    use crate::common::settings::TelemetrySettings;
    use opentelemetry::{api::Provider, global, sdk};
    use tracing_opentelemetry::OpentelemetryLayer;
    use tracing_subscriber::layer::Layer;

    fn init_tracer(settings: TelemetrySettings) {
        // Create a jaeger exporter for our service.
        let exporter = opentelemetry_jaeger::Exporter::builder()
            .with_agent_endpoint(settings.jaeger_endpoint)
            .with_process(opentelemetry_jaeger::Process {
                service_name: settings.service_name,
                tags: Vec::new(),
            })
            .init()
            .expect("Error initializing Jaeger exporter");

        // Build a provider from the jaeger exporter that always
        // samples.
        let provider = sdk::Provider::builder()
            .with_simple_exporter(exporter)
            .with_config(sdk::Config {
                default_sampler: Box::new(sdk::Sampler::Always),
                ..Default::default()
            })
            .build();

        global::set_provider(provider);
    }

    pub fn configure(settings: LoggingSettings) {
        // Redirect normal log messages to the tracing subscriber
        tracing_log::LogTracer::init().unwrap();

        let fmt_subscriber = FmtSubscriber::builder()
            .with_ansi(true)
            .with_timer(ChronoUtc::rfc3339())
            .with_env_filter(settings.filter)
            .finish();

        if let Some(telemetry_settings) = settings.telemetry {
            init_tracer(telemetry_settings);
            let tracer = global::trace_provider().get_tracer("tracer");
            let opentelemetry_subscriber = OpentelemetryLayer::with_tracer(tracer);
            let subscriber = opentelemetry_subscriber.with_subscriber(fmt_subscriber);
            tracing::subscriber::set_global_default(subscriber).expect("failed to setup tracing");
        } else {
            tracing::subscriber::set_global_default(fmt_subscriber)
                .expect("failed to setup tracing");
        };
    }
}

#[cfg(feature = "telemetry")]
pub use telemetry::configure;
