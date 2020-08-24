//! Provides functionality to enable clients to communicate with a XayNet
//! service over HTTP.
//!
//! See the [client module] documentation since this is a private module anyways.
//!
//! [client module]: ../index.html

use crate::{
    common::RoundParameters,
    crypto::ByteObject,
    mask::model::Model,
    services::{FetchError, Fetcher, PetMessageError, PetMessageHandler},
    state_machine::coordinator::RoundParameters,
    SumDict,
    SumParticipantPublicKey,
    UpdateSeedDict,
};
use reqwest::{self, Client, Response, StatusCode};
use thiserror::Error;

#[async_trait]
pub trait ApiClient {
    type Error: ::std::fmt::Debug + ::std::error::Error + 'static;

    /// Retrieve the current round parameters
    async fn get_round_params(&mut self) -> Result<RoundParameters, Self::Error>;

    /// Retrieve the current sum dictionary, if available
    async fn get_sums(&mut self) -> Result<Option<SumDict>, Self::Error>;

    /// Retrieve the current seed dictionary for the given sum
    /// participant, if available.
    async fn get_seeds(
        &mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<Option<UpdateSeedDict>, Self::Error>;

    /// Retrieve the current model/mask length, if available
    async fn get_mask_length(&mut self) -> Result<Option<u64>, Self::Error>;

    /// Retrieve the current global model, if available.
    async fn get_model(&mut self) -> Result<Option<Model>, Self::Error>;

    async fn send_message(&mut self, msg: Vec<u8>) -> Result<(), Self::Error>;
}

pub struct InMemoryApiClient {
    fetcher: Box<dyn Fetcher + Send + Sync>,
    message_handler: Box<dyn PetMessageHandler + Send + Sync>,
}

impl InMemoryApiClient {
    #[allow(dead_code)]
    pub fn new(
        fetcher: impl Fetcher + 'static + Send + Sync,
        message_handler: impl PetMessageHandler + 'static + Send + Sync,
    ) -> Self {
        Self {
            fetcher: Box::new(fetcher),
            message_handler: Box::new(message_handler),
        }
    }
}

#[derive(Debug, Error)]
pub enum InMemoryClientError {
    #[error("a PET message could not be processed by the coordinator: {0}")]
    Message(#[from] PetMessageError),

    #[error("failed to fetch data from the coordinator: {0}")]
    Fetch(#[from] FetchError),
}

#[async_trait]
impl ApiClient for InMemoryApiClient {
    type Error = InMemoryClientError;

    async fn get_round_params(&mut self) -> Result<RoundParameters, Self::Error> {
        Ok(self.fetcher.round_params().await?)
    }

    async fn get_sums(&mut self) -> Result<Option<SumDict>, Self::Error> {
        Ok(self.fetcher.sum_dict().await?.map(|arc| (*arc).clone()))
    }

    async fn get_seeds(
        &mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<Option<UpdateSeedDict>, Self::Error> {
        Ok(self
            .fetcher
            .seed_dict()
            .await?
            .and_then(|dict| dict.get(&pk).cloned()))
    }

    async fn get_mask_length(&mut self) -> Result<Option<u64>, Self::Error> {
        Ok(self.fetcher.mask_length().await?.map(|res| res as u64))
    }

    async fn get_model(&mut self) -> Result<Option<Model>, Self::Error> {
        Ok(self.fetcher.model().await?.map(|arc| (*arc).clone()))
    }

    async fn send_message(&mut self, message: Vec<u8>) -> Result<(), Self::Error> {
        Ok(self.message_handler.handle_message(message).await?)
    }
}

#[derive(Debug)]
/// Manages client requests over HTTP.
pub struct HttpApiClient {
    client: Client,
    address: String,
}

impl HttpApiClient {
    pub fn new<S>(address: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            client: Client::new(),
            address: address.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum HttpApiClientError {
    #[error("failed to deserialize data: {0}")]
    Deserialize(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Unexpected response from the coordinator: {:?}", .0)]
    UnexpectedResponse(Response),
}

impl From<bincode::Error> for HttpApiClientError {
    fn from(e: bincode::Error) -> Self {
        Self::Deserialize(format!("{:?}", e))
    }
}

impl From<::std::num::ParseIntError> for HttpApiClientError {
    fn from(e: ::std::num::ParseIntError) -> Self {
        Self::Deserialize(format!("{:?}", e))
    }
}

#[async_trait]
impl ApiClient for HttpApiClient {
    type Error = HttpApiClientError;

    async fn get_round_params(&mut self) -> Result<RoundParameters, Self::Error> {
        let url = format!("{}/params", self.address);
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        if let StatusCode::OK = resp.status() {
            let body = resp.bytes().await?; //
            Ok(bincode::deserialize(&body[..])?)
        } else {
            Err(HttpApiClientError::UnexpectedResponse(resp))
        }
    }

    async fn get_sums(&mut self) -> Result<Option<SumDict>, Self::Error> {
        let url = format!("{}/sums", self.address);
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        match resp.status() {
            StatusCode::OK => {
                let body = resp.bytes().await?;
                Ok(Some(bincode::deserialize(&body[..])?))
            }
            StatusCode::NO_CONTENT => Ok(None),
            _ => Err(HttpApiClientError::UnexpectedResponse(resp)),
        }
    }

    async fn get_seeds(
        &mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<Option<UpdateSeedDict>, Self::Error> {
        let url = format!("{}/seeds", self.address);
        let resp = self
            .client
            .get(&url)
            .header("Content-Type", "application/octet-stream")
            .body(pk.as_slice().to_vec())
            .send()
            .await?
            .error_for_status()?;
        match resp.status() {
            StatusCode::OK => {
                let body = resp.bytes().await?;
                Ok(Some(bincode::deserialize(&body[..])?))
            }
            StatusCode::NO_CONTENT => Ok(None),
            _ => Err(HttpApiClientError::UnexpectedResponse(resp)),
        }
    }

    async fn get_mask_length(&mut self) -> Result<Option<u64>, Self::Error> {
        let url = format!("{}/length", self.address);
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        match resp.status() {
            StatusCode::OK => Ok(Some(resp.text().await?.parse()?)),
            StatusCode::NO_CONTENT => Ok(None),
            _ => Err(HttpApiClientError::UnexpectedResponse(resp)),
        }
    }

    async fn get_model(&mut self) -> Result<Option<Model>, Self::Error> {
        let url = format!("{}/model", self.address);
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        match resp.status() {
            StatusCode::OK => {
                let body = resp.bytes().await?;
                Ok(Some(bincode::deserialize(&body[..])?))
            }
            StatusCode::NO_CONTENT => Ok(None),
            _ => Err(HttpApiClientError::UnexpectedResponse(resp)),
        }
    }

    async fn send_message(&mut self, msg: Vec<u8>) -> Result<(), Self::Error> {
        let url = format!("{}/message", self.address);
        self.client
            .post(&url)
            .body(msg)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
