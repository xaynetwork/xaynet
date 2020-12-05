use super::models::{Event, Metric};
use derive_more::From;
use influxdb::Client as InfluxClient;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;
use tracing::debug;

#[derive(From)]
pub(in crate::metrics) enum Request {
    Metric(Metric),
    Event(Event),
}

impl Request {
    fn into_query(self) -> influxdb::WriteQuery {
        match self {
            Request::Metric(metric) => metric.into_query(),
            Request::Event(event) => event.into_query(),
        }
    }
}

#[derive(Clone)]
pub(in crate::metrics) struct Dispatcher {
    client: InfluxClient,
}

impl Dispatcher {
    pub fn new(url: &str, database: &str) -> Self {
        let client = InfluxClient::new(url, database);
        Self { client }
    }
}

impl Service<Request> for Dispatcher {
    type Response = ();
    type Error = anyhow::Error;
    #[allow(clippy::type_complexity)]
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let client = self.client.clone();
        let fut = async move {
            debug!("dispatch metric");
            client
                .query(&req.into_query())
                .await
                .map_err(|err| anyhow::anyhow!("failed to dispatch metric {}", err))?;
            Ok(())
        };

        Box::pin(fut)
    }
}
