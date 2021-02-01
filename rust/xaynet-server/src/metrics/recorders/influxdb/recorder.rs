use std::borrow::Borrow;

use futures::future::poll_fn;
use influxdb::Type;
use tower::Service;
use tracing::{error, warn};

use super::{Dispatcher, Event, InfluxDbService, Measurement, Metric, Request, Tags};
use crate::settings::InfluxSettings;

/// An InfluxDB metrics / events recorder.
pub struct Recorder {
    /// A services that dispatches the recorded metrics / events to an InfluxDB instance.
    service: InfluxDbService,
}

impl Recorder {
    /// Creates a new InfluxDB recorder.
    pub fn new(settings: InfluxSettings) -> Self {
        let dispatcher = Dispatcher::new(settings.url, settings.db);
        Self {
            service: InfluxDbService::new(dispatcher),
        }
    }

    /// Records a new metric and dispatches it to an InfluxDB instance.
    pub fn metric(&self, measurement: Measurement, value: impl Into<Type>, tags: Option<Tags>) {
        let mut metric = Metric::new(measurement, value);
        if let Some(tags) = tags {
            metric.with_tags(tags);
        }

        self.call(metric.into())
    }

    /// Records a new event and dispatches it to an InfluxDB instance.
    pub fn event(
        &self,
        title: impl Into<String>,
        description: Option<impl Into<String>>,
        tags: Option<&[impl Borrow<str>]>,
    ) {
        let mut event = Event::new(title);

        if let Some(description) = description {
            event.with_description(description);
        }

        if let Some(tags) = tags {
            event.with_tags(tags);
        }

        self.call(event.into())
    }

    fn call(&self, req: Request) {
        let mut handle = self.service.0.clone();
        tokio::spawn(async move {
            if let Err(err) = poll_fn(|cx| handle.poll_ready(cx)).await {
                error!("influx service temporarily unavailable: {}", err)
            }

            if let Err(err) = handle.call(req).await {
                warn!("influx service error: {}", err)
            }
        });
    }
}
