mod mask_length;
mod model;
mod round_parameters;
mod scalar;
mod seed_dict;
mod sum_dict;

pub use self::{
    mask_length::{MaskLengthRequest, MaskLengthResponse, MaskLengthService},
    model::{ModelRequest, ModelResponse, ModelService},
    round_parameters::{RoundParamsRequest, RoundParamsResponse, RoundParamsService},
    scalar::{ScalarRequest, ScalarResponse, ScalarService},
    seed_dict::{SeedDictRequest, SeedDictResponse, SeedDictService},
    sum_dict::{SumDictRequest, SumDictResponse, SumDictService},
};

use std::task::{Context, Poll};

use futures::future::poll_fn;
use tower::Service;

use crate::state_machine::coordinator::RoundParameters;

#[async_trait]
pub trait Fetcher {
    async fn round_params(&self) -> Result<RoundParamsResponse, FetchError>;
    async fn mask_length(&self) -> Result<MaskLengthResponse, FetchError>;
    async fn scalar(&self) -> Result<ScalarResponse, FetchError>;
    async fn model(&self) -> Result<ModelResponse, FetchError>;
    async fn seed_dict(&self) -> Result<SeedDictResponse, FetchError>;
    async fn sum_dict(&self) -> Result<SumDictResponse, FetchError>;
}

pub type FetchError = anyhow::Error;

fn into_fetch_error<E: Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>>(
    e: E,
) -> FetchError {
    anyhow::anyhow!("Fetcher failed: {:?}", e.into())
}

#[async_trait]
impl<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model> Fetcher
    for FetcherService<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model>
where
    Self: Clone
        + Send
        + Sync
        + 'static
        + Service<RoundParamsRequest, Response = RoundParameters>
        + Service<MaskLengthRequest, Response = MaskLengthResponse>
        + Service<ScalarRequest, Response = ScalarResponse>
        + Service<ModelRequest, Response = ModelResponse>
        + Service<SeedDictRequest, Response = SeedDictResponse>
        + Service<SumDictRequest, Response = SumDictResponse>,

    <Self as Service<RoundParamsRequest>>::Future: Send + Sync + 'static,
    <Self as Service<RoundParamsRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,

    <Self as Service<MaskLengthRequest>>::Future: Send + Sync + 'static,
    <Self as Service<MaskLengthRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,

    <Self as Service<ScalarRequest>>::Future: Send + Sync + 'static,
    <Self as Service<ScalarRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,

    <Self as Service<ModelRequest>>::Future: Send + Sync + 'static,
    <Self as Service<ModelRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,

    <Self as Service<SeedDictRequest>>::Future: Send + Sync + 'static,
    <Self as Service<SeedDictRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,

    <Self as Service<SumDictRequest>>::Future: Send + Sync + 'static,
    <Self as Service<SumDictRequest>>::Error:
        Into<Box<dyn ::std::error::Error + 'static + Sync + Send>>,
{
    async fn round_params(&self) -> Result<RoundParameters, FetchError> {
        let mut svc = self.clone();
        poll_fn(|cx| <Self as Service<RoundParamsRequest>>::poll_ready(&mut svc, cx))
            .await
            .map_err(into_fetch_error)?;
        Ok(
            <Self as Service<RoundParamsRequest>>::call(&mut svc, RoundParamsRequest)
                .await
                .map_err(into_fetch_error)?,
        )
    }

    async fn mask_length(&self) -> Result<MaskLengthResponse, FetchError> {
        let mut svc = self.clone();
        poll_fn(|cx| <Self as Service<MaskLengthRequest>>::poll_ready(&mut svc, cx))
            .await
            .map_err(into_fetch_error)?;
        Ok(
            <Self as Service<MaskLengthRequest>>::call(&mut svc, MaskLengthRequest)
                .await
                .map_err(into_fetch_error)?,
        )
    }

    async fn scalar(&self) -> Result<ScalarResponse, FetchError> {
        let mut svc = self.clone();
        poll_fn(|cx| <Self as Service<ScalarRequest>>::poll_ready(&mut svc, cx))
            .await
            .map_err(into_fetch_error)?;
        Ok(
            <Self as Service<ScalarRequest>>::call(&mut svc, ScalarRequest)
                .await
                .map_err(into_fetch_error)?,
        )
    }

    async fn model(&self) -> Result<ModelResponse, FetchError> {
        let mut svc = self.clone();
        poll_fn(|cx| <Self as Service<ModelRequest>>::poll_ready(&mut svc, cx))
            .await
            .map_err(into_fetch_error)?;
        Ok(
            <Self as Service<ModelRequest>>::call(&mut svc, ModelRequest)
                .await
                .map_err(into_fetch_error)?,
        )
    }

    async fn seed_dict(&self) -> Result<SeedDictResponse, FetchError> {
        let mut svc = self.clone();
        poll_fn(|cx| <Self as Service<SeedDictRequest>>::poll_ready(&mut svc, cx))
            .await
            .map_err(into_fetch_error)?;
        Ok(
            <Self as Service<SeedDictRequest>>::call(&mut svc, SeedDictRequest)
                .await
                .map_err(into_fetch_error)?,
        )
    }

    async fn sum_dict(&self) -> Result<SumDictResponse, FetchError> {
        let mut svc = self.clone();
        poll_fn(|cx| <Self as Service<SumDictRequest>>::poll_ready(&mut svc, cx))
            .await
            .map_err(into_fetch_error)?;
        Ok(
            <Self as Service<SumDictRequest>>::call(&mut svc, SumDictRequest)
                .await
                .map_err(into_fetch_error)?,
        )
    }
}

/// A service for fetching PET data.
#[derive(Clone)]
pub struct FetcherService<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model> {
    round_params: RoundParams,
    sum_dict: SumDict,
    seed_dict: SeedDict,
    mask_length: MaskLength,
    scalar: Scalar,
    model: Model,
}

impl<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model>
    FetcherService<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model>
{
    pub fn new(
        round_params: RoundParams,
        sum_dict: SumDict,
        seed_dict: SeedDict,
        mask_length: MaskLength,
        scalar: Scalar,
        model: Model,
    ) -> Self {
        Self {
            round_params,
            sum_dict,
            seed_dict,
            mask_length,
            scalar,
            model,
        }
    }
}

impl<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model> Service<SumDictRequest>
    for FetcherService<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model>
where
    SumDict: Service<SumDictRequest>,
{
    type Response = <SumDict as Service<SumDictRequest>>::Response;
    type Error = <SumDict as Service<SumDictRequest>>::Error;
    type Future = <SumDict as Service<SumDictRequest>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <SumDict as Service<SumDictRequest>>::poll_ready(&mut self.sum_dict, cx)
    }

    fn call(&mut self, req: SumDictRequest) -> Self::Future {
        <SumDict as Service<SumDictRequest>>::call(&mut self.sum_dict, req)
    }
}

impl<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model> Service<SeedDictRequest>
    for FetcherService<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model>
where
    SeedDict: Service<SeedDictRequest>,
{
    type Response = <SeedDict as Service<SeedDictRequest>>::Response;
    type Error = <SeedDict as Service<SeedDictRequest>>::Error;
    type Future = <SeedDict as Service<SeedDictRequest>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <SeedDict as Service<SeedDictRequest>>::poll_ready(&mut self.seed_dict, cx)
    }

    fn call(&mut self, req: SeedDictRequest) -> Self::Future {
        <SeedDict as Service<SeedDictRequest>>::call(&mut self.seed_dict, req)
    }
}

impl<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model> Service<RoundParamsRequest>
    for FetcherService<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model>
where
    RoundParams: Service<RoundParamsRequest>,
{
    type Response = <RoundParams as Service<RoundParamsRequest>>::Response;
    type Error = <RoundParams as Service<RoundParamsRequest>>::Error;
    type Future = <RoundParams as Service<RoundParamsRequest>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <RoundParams as Service<RoundParamsRequest>>::poll_ready(&mut self.round_params, cx)
    }

    fn call(&mut self, req: RoundParamsRequest) -> Self::Future {
        <RoundParams as Service<RoundParamsRequest>>::call(&mut self.round_params, req)
    }
}

impl<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model> Service<ScalarRequest>
    for FetcherService<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model>
where
    Scalar: Service<ScalarRequest>,
{
    type Response = <Scalar as Service<ScalarRequest>>::Response;
    type Error = <Scalar as Service<ScalarRequest>>::Error;
    type Future = <Scalar as Service<ScalarRequest>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <Scalar as Service<ScalarRequest>>::poll_ready(&mut self.scalar, cx)
    }

    fn call(&mut self, req: ScalarRequest) -> Self::Future {
        <Scalar as Service<ScalarRequest>>::call(&mut self.scalar, req)
    }
}

impl<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model> Service<ModelRequest>
    for FetcherService<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model>
where
    Model: Service<ModelRequest>,
{
    type Response = <Model as Service<ModelRequest>>::Response;
    type Error = <Model as Service<ModelRequest>>::Error;
    type Future = <Model as Service<ModelRequest>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <Model as Service<ModelRequest>>::poll_ready(&mut self.model, cx)
    }

    fn call(&mut self, req: ModelRequest) -> Self::Future {
        <Model as Service<ModelRequest>>::call(&mut self.model, req)
    }
}

impl<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model> Service<MaskLengthRequest>
    for FetcherService<RoundParams, SumDict, SeedDict, MaskLength, Scalar, Model>
where
    MaskLength: Service<MaskLengthRequest>,
{
    type Response = <MaskLength as Service<MaskLengthRequest>>::Response;
    type Error = <MaskLength as Service<MaskLengthRequest>>::Error;
    type Future = <MaskLength as Service<MaskLengthRequest>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <MaskLength as Service<MaskLengthRequest>>::poll_ready(&mut self.mask_length, cx)
    }

    fn call(&mut self, req: MaskLengthRequest) -> Self::Future {
        <MaskLength as Service<MaskLengthRequest>>::call(&mut self.mask_length, req)
    }
}
