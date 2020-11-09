//! This module provides the services for serving data.
//!
//! There are multiple such services and the [`Fetcher`] trait
//! provides a single unifying interface for all of these.

mod mask_length;
mod model;
mod round_parameters;
mod seed_dict;
mod sum_dict;

use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::future::poll_fn;
use tower::{layer::Layer, Service, ServiceBuilder};

pub use self::{
    mask_length::{MaskLengthRequest, MaskLengthResponse, MaskLengthService},
    model::{ModelRequest, ModelResponse, ModelService},
    round_parameters::{RoundParamsRequest, RoundParamsResponse, RoundParamsService},
    seed_dict::{SeedDictRequest, SeedDictResponse, SeedDictService},
    sum_dict::{SumDictRequest, SumDictResponse, SumDictService},
};
use crate::state_machine::events::EventSubscriber;

/// A single interface for retrieving data from the coordinator.
#[async_trait]
pub trait Fetcher {
    /// Fetch the parameters for the current round
    async fn round_params(&mut self) -> Result<RoundParamsResponse, FetchError>;

    /// Fetch the mask length for the current round. The sum
    /// participants need this value during the sum2 phase to derive
    /// masks from the update participant's masking seeds.
    async fn mask_length(&mut self) -> Result<MaskLengthResponse, FetchError>;

    /// Fetch the latest global model.
    async fn model(&mut self) -> Result<ModelResponse, FetchError>;

    /// Fetch the global seed dictionary. Each sum2 participant needs a
    /// different portion of that dictionary.
    async fn seed_dict(&mut self) -> Result<SeedDictResponse, FetchError>;

    /// Fetch the sum dictionary. The update participants need this
    /// dictionary to encrypt their masking seed for each sum
    /// participant.
    async fn sum_dict(&mut self) -> Result<SumDictResponse, FetchError>;
}

/// An error returned by the [`Fetcher`]'s method.
pub type FetchError = anyhow::Error;

fn into_fetch_error<E: Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>>(
    e: E,
) -> FetchError {
    anyhow::anyhow!("Fetcher failed: {:?}", e.into())
}

#[async_trait]
impl<RoundParams, SumDict, SeedDict, MaskLength, Model> Fetcher
    for Fetchers<RoundParams, SumDict, SeedDict, MaskLength, Model>
where
    Self: Send + Sync + 'static,

    RoundParams: Service<RoundParamsRequest, Response = RoundParamsResponse> + Send + 'static,
    <RoundParams as Service<RoundParamsRequest>>::Future: Send + Sync + 'static,
    <RoundParams as Service<RoundParamsRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,

    MaskLength: Service<MaskLengthRequest, Response = MaskLengthResponse> + Send + 'static,
    <MaskLength as Service<MaskLengthRequest>>::Future: Send + Sync + 'static,
    <MaskLength as Service<MaskLengthRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,

    Model: Service<ModelRequest, Response = ModelResponse> + Send + 'static,
    <Model as Service<ModelRequest>>::Future: Send + Sync + 'static,
    <Model as Service<ModelRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,

    SeedDict: Service<SeedDictRequest, Response = SeedDictResponse> + Send + 'static,
    <SeedDict as Service<SeedDictRequest>>::Future: Send + Sync + 'static,
    <SeedDict as Service<SeedDictRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,

    SumDict: Service<SumDictRequest, Response = SumDictResponse> + Send + 'static,
    <SumDict as Service<SumDictRequest>>::Future: Send + Sync + 'static,
    <SumDict as Service<SumDictRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,
{
    async fn round_params(&mut self) -> Result<RoundParamsResponse, FetchError> {
        poll_fn(|cx| {
            <RoundParams as Service<RoundParamsRequest>>::poll_ready(&mut self.round_params, cx)
        })
        .await
        .map_err(into_fetch_error)?;
        Ok(<RoundParams as Service<RoundParamsRequest>>::call(
            &mut self.round_params,
            RoundParamsRequest,
        )
        .await
        .map_err(into_fetch_error)?)
    }

    async fn mask_length(&mut self) -> Result<MaskLengthResponse, FetchError> {
        poll_fn(|cx| {
            <MaskLength as Service<MaskLengthRequest>>::poll_ready(&mut self.mask_length, cx)
        })
        .await
        .map_err(into_fetch_error)?;
        Ok(<MaskLength as Service<MaskLengthRequest>>::call(
            &mut self.mask_length,
            MaskLengthRequest,
        )
        .await
        .map_err(into_fetch_error)?)
    }

    async fn model(&mut self) -> Result<ModelResponse, FetchError> {
        poll_fn(|cx| <Model as Service<ModelRequest>>::poll_ready(&mut self.model, cx))
            .await
            .map_err(into_fetch_error)?;
        Ok(
            <Model as Service<ModelRequest>>::call(&mut self.model, ModelRequest)
                .await
                .map_err(into_fetch_error)?,
        )
    }

    async fn seed_dict(&mut self) -> Result<SeedDictResponse, FetchError> {
        poll_fn(|cx| <SeedDict as Service<SeedDictRequest>>::poll_ready(&mut self.seed_dict, cx))
            .await
            .map_err(into_fetch_error)?;
        Ok(
            <SeedDict as Service<SeedDictRequest>>::call(&mut self.seed_dict, SeedDictRequest)
                .await
                .map_err(into_fetch_error)?,
        )
    }

    async fn sum_dict(&mut self) -> Result<SumDictResponse, FetchError> {
        poll_fn(|cx| <SumDict as Service<SumDictRequest>>::poll_ready(&mut self.sum_dict, cx))
            .await
            .map_err(into_fetch_error)?;
        Ok(
            <SumDict as Service<SumDictRequest>>::call(&mut self.sum_dict, SumDictRequest)
                .await
                .map_err(into_fetch_error)?,
        )
    }
}

pub(in crate::services) struct FetcherService<S>(S);

impl<S, R> Service<R> for FetcherService<S>
where
    S: Service<R>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, req: R) -> Self::Future {
        self.0.call(req)
    }
}

pub(in crate::services) struct FetcherLayer;

impl<S> Layer<S> for FetcherLayer {
    type Service = FetcherService<S>;

    fn layer(&self, service: S) -> Self::Service {
        FetcherService(service)
    }
}

#[derive(Debug, Clone)]
pub struct Fetchers<RoundParams, SumDict, SeedDict, MaskLength, Model> {
    round_params: RoundParams,
    sum_dict: SumDict,
    seed_dict: SeedDict,
    mask_length: MaskLength,
    model: Model,
}

impl<RoundParams, SumDict, SeedDict, MaskLength, Model>
    Fetchers<RoundParams, SumDict, SeedDict, MaskLength, Model>
{
    pub fn new(
        round_params: RoundParams,
        sum_dict: SumDict,
        seed_dict: SeedDict,
        mask_length: MaskLength,
        model: Model,
    ) -> Self {
        Self {
            round_params,
            sum_dict,
            seed_dict,
            mask_length,
            model,
        }
    }
}

/// Construct a [`Fetcher`] service
pub fn fetcher(event_subscriber: &EventSubscriber) -> impl Fetcher + Sync + Send + Clone + 'static {
    let round_params = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .layer(FetcherLayer)
        .service(RoundParamsService::new(event_subscriber));

    let mask_length = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .layer(FetcherLayer)
        .service(MaskLengthService::new(event_subscriber));

    let model = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .layer(FetcherLayer)
        .service(ModelService::new(event_subscriber));

    let sum_dict = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .layer(FetcherLayer)
        .service(SumDictService::new(event_subscriber));

    let seed_dict = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .layer(FetcherLayer)
        .service(SeedDictService::new(event_subscriber));

    Fetchers::new(round_params, sum_dict, seed_dict, mask_length, model)
}
