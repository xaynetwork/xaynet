use crate::{metrics, settings::MetricsSettings};

pub fn init(settings: MetricsSettings) -> Result<(), ()> {
    tracing::debug!("initialize");
    let recorder = metrics::Recorder::new(settings.influxdb);
    metrics::GlobalRecorder::install(recorder).map_err(|_| ())
}
