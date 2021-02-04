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
    pub fn metric<V, T, I>(&self, measurement: Measurement, value: V, tags: T)
    where
        V: Into<Type>,
        T: Into<Option<I>>,
        I: Into<Tags>,
    {
        let metric = Metric::new(measurement, value).with_tags(tags);
        self.call(metric.into());
    }

    /// Records a new event and dispatches it to an InfluxDB instance.
    pub fn event<H, D, S, T, A, B>(&self, title: H, description: D, tags: T)
    where
        H: Into<String>,
        D: Into<Option<S>>,
        S: Into<String>,
        T: Into<Option<A>>,
        A: AsRef<[B]>,
        B: Borrow<str>,
    {
        let event = Event::new(title)
            .with_description(description)
            .with_tags(tags);
        self.call(event.into());
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
