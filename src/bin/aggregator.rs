#[macro_use]
extern crate async_trait;
use bytes::Bytes;
use futures::future::{ready, Ready};

use xain_fl::aggregator::service::{Aggregator, AggregatorService};

#[tokio::main]
async fn main() {
    _main().await;
}

async fn _main() {
    env_logger::init();
    let aggregator = AggregatorService::new(DummyAggregator, "localhost:6666", "localhost:5555");
    aggregator.await;
}

struct DummyAggregator;

#[async_trait]
impl Aggregator for DummyAggregator {
    type Error = ::std::io::Error;
    type AggregateFut = Ready<Result<Bytes, Self::Error>>;

    async fn add_weights(&mut self, _weights: Bytes) -> Result<(), Self::Error> {
        ready(Ok(())).await
    }
    fn aggregate(&mut self) -> Self::AggregateFut {
        ready(Ok(Bytes::new()))
    }
}
