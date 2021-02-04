use super::models::{Event, Metric};
use derive_more::From;
use futures::future::BoxFuture;
use influxdb::{Client as InfluxClient, WriteQuery};
use std::task::{Context, Poll};
use tower::Service;
use tracing::debug;

#[derive(From)]
pub(in crate::metrics) enum Request {
    Metric(Metric),
    Event(Event),
}

impl From<Request> for WriteQuery {
    fn from(req: Request) -> Self {
        match req {
            Request::Metric(metric) => metric.into(),
            Request::Event(event) => event.into(),
        }
    }
}

#[derive(Clone)]
pub(in crate::metrics) struct Dispatcher {
    client: InfluxClient,
}

impl Dispatcher {
    pub fn new(url: impl Into<String>, database: impl Into<String>) -> Self {
        let client = InfluxClient::new(url, database);
        Self { client }
    }
}

impl Service<Request> for Dispatcher {
    type Response = ();
    type Error = anyhow::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let client = self.client.clone();
        let fut = async move {
            debug!("dispatch metric");
            client
                .query(&WriteQuery::from(req))
                .await
                .map_err(|err| anyhow::anyhow!("failed to dispatch metric {}", err))?;
            Ok(())
        };

        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use tokio_test::assert_ready;
    use tower_test::mock::Spawn;

    use super::*;
    use crate::{
        metrics::{
            recorders::influxdb::models::{Event, Metric},
            Measurement,
        },
        settings::InfluxSettings,
    };

    fn influx_settings() -> InfluxSettings {
        InfluxSettings {
            url: "http://127.0.0.1:8086".to_string(),
            db: "metrics".to_string(),
        }
    }

    #[tokio::test]
    async fn integration_dispatch_metric() {
        let settings = influx_settings();
        let mut task = Spawn::new(Dispatcher::new(settings.url, settings.db));

        let metric = Metric::new(Measurement::Phase, 1);
        assert_ready!(task.poll_ready()).unwrap();
        let resp = task.call(metric.into()).await;
        assert!(resp.is_ok());
    }

    #[tokio::test]
    async fn integration_dispatch_event() {
        let settings = influx_settings();
        let mut task = Spawn::new(Dispatcher::new(settings.url, settings.db));

        let event = Event::new("event");
        assert_ready!(task.poll_ready()).unwrap();
        let resp = task.call(event.into()).await;
        assert!(resp.is_ok());
    }

    #[tokio::test]
    async fn integration_wrong_url() {
        let settings = influx_settings();
        let mut task = Spawn::new(Dispatcher::new("http://127.0.0.1:9998", settings.db));

        let event = Event::new("event");
        assert_ready!(task.poll_ready()).unwrap();
        let resp = task.call(event.into()).await;
        assert!(resp.is_err());
    }
}
