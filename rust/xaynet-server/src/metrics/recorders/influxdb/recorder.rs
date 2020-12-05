use super::{Dispatcher, Event, InfluxDbService, Measurement, Metric, Request, Tags};
use crate::settings::InfluxSettings;

use futures::future::poll_fn;
use tower::Service;
use tracing::{error, warn};

/// An InfluxDB metrics / events recorder.
pub struct Recorder {
    /// A services that dispatches the recorded metrics / events to an InfluxDB instance.
    service: InfluxDbService,
}

impl Recorder {
    /// Creates a new InfluxDB recorder.
    pub fn new(settings: InfluxSettings) -> Self {
        let dispatcher = Dispatcher::new(&settings.url, &settings.db);
        Self {
            service: InfluxDbService::new(dispatcher),
        }
    }

    /// Records a new metric and dispatches it to an InfluxDB instance.
    pub fn metric<V>(&self, measurement: Measurement, value: V, tags: Option<Tags>)
    where
        V: Into<influxdb::Type> + Send + 'static,
    {
        let mut metric = Metric::new(measurement, value.into());
        if let Some(tags) = tags {
            metric.with_tags(tags);
        }

        self.call(Request::from(metric))
    }

    /// Records a new event and dispatches it to an InfluxDB instance.
    pub fn event<'a, T>(&self, title: T, description: Option<&str>, tags: Option<&'a [&'a str]>)
    where
        T: Into<String> + Send + 'static,
    {
        let mut event = Event::new(title.into());

        if let Some(description) = description {
            event.with_description(description);
        }

        if let Some(tags) = tags {
            event.with_tags(tags);
        }

        self.call(Request::from(event))
    }

    fn call(&self, req: Request) {
        let mut handle = self.service.0.clone();
        tokio::spawn(async move {
            if poll_fn(|cx| handle.poll_ready(cx)).await.is_err() {
                error!("influx service failed")
            }

            if let Err(err) = handle.call(req).await {
                warn!("influx service error: {}", err)
            }
        });
    }
}
