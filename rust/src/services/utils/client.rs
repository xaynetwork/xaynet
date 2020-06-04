use std::{
    fmt::Debug,
    task::{Context, Poll},
};

use futures::{future::MapErr, Sink, TryFutureExt, TryStream};
use thiserror::Error;
use tokio_tower::pipeline::client::Client as TowerClient;
use tower::{buffer::Buffer, Service, ServiceBuilder};

use crate::services::{error::ServiceError, utils::trace::Traced};

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Service internal error: {0:?}")]
    Service(#[from] ServiceError),
}

impl<T, I> From<tokio_tower::Error<T, I>> for ClientError
where
    T: Sink<I> + TryStream,
    <T as Sink<I>>::Error: Debug,
    <T as TryStream>::Error: Debug,
{
    fn from(e: tokio_tower::Error<T, I>) -> Self {
        Self::Transport(format!("{:?}", e))
    }
}

/// A clonable client for sending traced requests.
pub struct Client<T, Req, Resp>
where
    Req: Debug + 'static + Send,
    Resp: Debug + 'static + Send,
    T: Sink<Traced<Req>> + TryStream<Ok = Resp> + 'static + Send,
    <T as Sink<Traced<Req>>>::Error: Debug + 'static + Send,
    <T as TryStream>::Ok: Debug + 'static + Send,
    <T as TryStream>::Error: Debug + 'static + Send,
{
    inner: Buffer<TowerClient<T, ClientError, Traced<Req>>, Traced<Req>>,
}

impl<T, Req, Resp> ::std::clone::Clone for Client<T, Req, Resp>
where
    Req: Debug + 'static + Send,
    Resp: Debug + 'static + Send,
    T: Sink<Traced<Req>> + TryStream<Ok = Resp> + 'static + Send,
    <T as Sink<Traced<Req>>>::Error: Debug + 'static + Send,
    <T as TryStream>::Ok: Debug + 'static + Send,
    <T as TryStream>::Error: Debug + 'static + Send,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T, Req, Resp> Client<T, Req, Resp>
where
    Req: Debug + 'static + Send,
    Resp: Debug + 'static + Send,
    T: Sink<Traced<Req>> + TryStream<Ok = Resp> + 'static + Send,
    <T as Sink<Traced<Req>>>::Error: Debug + 'static + Send,
    <T as TryStream>::Ok: Debug + 'static + Send,
    <T as TryStream>::Error: Debug + 'static + Send,
{
    pub fn new(transport: T) -> Self {
        Self {
            inner: ServiceBuilder::new()
                .buffer(1000)
                .service(TowerClient::new(transport)),
        }
    }
}

impl<T, Req, Resp> Service<Traced<Req>> for Client<T, Req, Resp>
where
    Req: Debug + 'static + Send,
    Resp: Debug + 'static + Send,
    T: Sink<Traced<Req>> + TryStream<Ok = Resp> + 'static + Send,
    <T as Sink<Traced<Req>>>::Error: Debug + 'static + Send,
    <T as TryStream>::Error: Debug + 'static + Send,
    <T as TryStream>::Ok: Debug + 'static + Send,
{
    type Response = T::Ok;
    type Error = ClientError;
    type Future =
        MapErr<
            <Buffer<TowerClient<T, ClientError, Traced<Req>>, Traced<Req>> as Service<
                Traced<Req>,
            >>::Future,
            fn(Box<dyn std::error::Error + Send + Sync>) -> Self::Error,
        >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready(cx)
            // UNWRAP_SAFE: we know the error returned by a
            // TowerClient<_, ClientError, _> is a ClientError, so
            // downcasting should not fail
            .map_err(|std_err| *(std_err.downcast::<ClientError>().unwrap()))
    }

    fn call(&mut self, req: Traced<Req>) -> Self::Future {
        self.inner
            .call(req)
            // UNWRAP_SAFE: we know the error returned by a
            // TowerClient<_, ClientError, _> is a ClientError, so
            // downcasting should not fail
            .map_err(|std_err| *(std_err.downcast::<ClientError>().unwrap()))
    }
}
