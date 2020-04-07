use crate::{
    aggregator::service::{
        Aggregator, DownloadError, ServiceError, ServiceHandle as InnerServiceHandle,
        ServiceRequests, UploadError,
    },
    common::client::Credentials,
};
use bytes::Bytes;
use futures::future;
use thiserror::Error;

#[derive(Clone)]
pub struct ServiceHandle<A: Aggregator>(InnerServiceHandle<A>);

pub struct ByteAggregator {
    weights: Vec<u8>,
}

impl ByteAggregator {
    pub fn new() -> ByteAggregator {
        ByteAggregator { weights: vec![] }
    }
}

#[derive(Debug, Error)]
#[error("dummy error")]
pub struct ByteAggregatorError;

impl Aggregator for ByteAggregator {
    type Error = ByteAggregatorError;

    type AddWeightsFut = future::Ready<Result<(), Self::Error>>;
    type AggregateFut = future::Ready<Result<Bytes, Self::Error>>;

    fn add_weights(&mut self, weights: Bytes) -> Self::AddWeightsFut {
        self.weights.extend(weights.into_iter());
        future::ready(Ok(()))
    }

    fn aggregate(&mut self) -> Self::AggregateFut {
        self.weights.sort();
        let global_weights = Bytes::copy_from_slice(&self.weights[..]);
        future::ready(Ok(global_weights))
    }
}

impl<A> ServiceHandle<A>
where
    A: Aggregator + 'static,
{
    pub fn new() -> (Self, ServiceRequests<A>) {
        let (inner, requests) = InnerServiceHandle::new();
        (Self(inner), requests)
    }

    pub async fn download(
        &self,
        credentials: Credentials,
    ) -> Result<Bytes, ServiceError<DownloadError>> {
        self.0.download(credentials).await
    }

    pub async fn upload(
        &self,
        credentials: Credentials,
        data: Bytes,
    ) -> Result<(), ServiceError<UploadError>> {
        self.0.upload(credentials, data).await
    }

    pub async fn aggregate(&self) -> Result<(), ServiceError<A::Error>> {
        self.0.aggregate().await
    }

    pub async fn select(&self, credentials: Credentials) -> Result<(), ServiceError<A::Error>> {
        self.0.select(credentials).await
    }
}
