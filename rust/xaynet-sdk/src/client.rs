use async_trait::async_trait;
use thiserror::Error;
use url::Url;

use xaynet_core::{
    common::RoundParameters,
    crypto::{ByteObject, PublicSigningKey},
    mask::Model,
    SumDict,
    UpdateSeedDict,
};

use crate::XaynetClient;

/// Error returned upon failing to build a new [`Client`]
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("failed to deserialize data: {0}")]
    Deserialize(String),

    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("{0}")]
    Other(String),

    #[error("Reading from file failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unexpected response")]
    UnexpectedResponse(u16),

    #[error("Unexpected certificate extension")]
    UnexpectedCertificate,

    #[error("No certificate found")]
    NoCertificate,
}

impl ClientError {
    fn http_error<E: std::error::Error>(e: E) -> Self {
        Self::Http(format!("{}", e))
    }
}

impl From<bincode::Error> for ClientError {
    fn from(e: bincode::Error) -> Self {
        Self::Deserialize(format!("{}", e))
    }
}

impl From<::std::num::ParseIntError> for ClientError {
    fn from(e: ::std::num::ParseIntError) -> Self {
        Self::Deserialize(format!("{}", e))
    }
}

/// A basic HTTP interface that [`Client`] HTTP backends must implement.
#[async_trait]
pub trait XaynetHttpClient {
    /// Error type for all the trait's methods
    type Error: std::error::Error;
    /// Reponse type for `GET` requests
    type GetResponse: AsRef<[u8]>;

    /// Perform an HTTP `GET` on the given URL.
    ///
    /// If the response is `NO_CONTENT`, the implementor must return `Ok(None)`. Otherwise, the
    /// response body must be returned
    async fn get(&mut self, url: &str) -> Result<Option<Self::GetResponse>, ClientError>;

    /// Perform an HTTP `POST` on the given URL, with the given body.
    async fn post(&mut self, url: &str, body: Vec<u8>) -> Result<(), ClientError>;
}

#[derive(Debug, Clone)]
/// A client that communicates with the coordinator's API via HTTP(S).
pub struct Client<C> {
    /// HTTP(S) client
    client: C,
    /// Coordinator URL
    base_url: Url,
}

/// Error returned when trying to client a [`Client`] with an invalid
/// address for the Xaynet coordinator.
#[derive(Debug, Error)]
#[error("Invalid base URL: {}", .0)]
pub struct InvalidBaseUrl(String);

impl<C> Client<C>
where
    C: XaynetHttpClient,
{
    /// Create a new client.
    ///
    /// # Args
    ///
    /// - `client` is the HTTP client that will be used to perform the HTTP requests. Any HTTP
    ///   client can be used, as long as it implements the [`XaynetHttpClient`] trait.
    /// - `base_url` is the URL to the Xaynet coordinator
    ///
    /// # Errors
    ///
    /// An error is returned if `base_url` is not a valid URL
    pub fn new(http_client: C, base_url: &str) -> Result<Self, InvalidBaseUrl> {
        let base_url = Url::parse(base_url).map_err(|e| InvalidBaseUrl(format!("{}", e)))?;
        if base_url.cannot_be_a_base() {
            return Err(InvalidBaseUrl(String::from("cannot be a base URL")));
        }
        Ok(Self {
            client: http_client,
            base_url,
        })
    }

    /// Append the given segment to the client base URL
    fn url(&self, segment: &str) -> Url {
        let mut url = self.base_url.clone();
        url.path_segments_mut().unwrap().push(segment);
        url
    }

    async fn get<T>(&mut self, url: &Url) -> Result<Option<T>, ClientError>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        Ok(match self.client.get(url.as_str()).await? {
            Some(data) => Some(bincode::deserialize::<T>(data.as_ref())?),
            None => None,
        })
    }

    async fn post(&mut self, url: &Url, data: Vec<u8>) -> Result<(), ClientError> {
        self.client.post(url.as_str(), data).await
    }
}

#[async_trait]
impl<C> XaynetClient for Client<C>
where
    C: XaynetHttpClient + Send,
{
    type Error = ClientError;

    async fn get_round_params(&mut self) -> Result<RoundParameters, Self::Error> {
        let url = self.url("params");
        let round_params: Option<RoundParameters> = self.get(&url).await?;
        round_params.ok_or_else(|| {
            ClientError::Other("failed to fetch round parameters: empty response".to_string())
        })
    }

    async fn get_sums(&mut self) -> Result<Option<SumDict>, Self::Error> {
        let url = self.url("sums");
        Ok(self.get(&url).await?)
    }

    async fn get_seeds(
        &mut self,
        pk: PublicSigningKey,
    ) -> Result<Option<UpdateSeedDict>, Self::Error> {
        let mut url = self.url("seeds");
        url.query_pairs_mut()
            .append_pair("pk", &base64::encode(pk.as_slice()));
        self.get(&url).await
    }

    async fn get_mask_length(&mut self) -> Result<Option<u64>, Self::Error> {
        let url = self.url("length");
        Ok(self.get(&url).await?)
    }

    async fn get_model(&mut self) -> Result<Option<Model>, Self::Error> {
        let url = self.url("model");
        Ok(self.get(&url).await?)
    }

    async fn send_message(&mut self, msg: Vec<u8>) -> Result<(), Self::Error> {
        let url = self.url("message");
        self.post(&url, msg).await
    }
}

#[cfg(feature = "reqwest-client")]
#[async_trait]
impl XaynetHttpClient for reqwest::Client {
    type Error = reqwest::Error;
    type GetResponse = bytes::Bytes;

    async fn get(&mut self, url: &str) -> Result<Option<Self::GetResponse>, ClientError> {
        let resp = reqwest::Client::get(self, url)
            .send()
            .await
            .map_err(ClientError::http_error)?
            .error_for_status()
            .map_err(ClientError::http_error)?;
        match resp.status() {
            reqwest::StatusCode::OK => {
                Ok(Some(resp.bytes().await.map_err(ClientError::http_error)?))
            }
            reqwest::StatusCode::NO_CONTENT => Ok(None),
            status => Err(ClientError::UnexpectedResponse(status.as_u16())),
        }
    }

    async fn post(&mut self, url: &str, body: Vec<u8>) -> Result<(), ClientError> {
        let _resp = reqwest::Client::post(self, url)
            .body(body)
            .send()
            .await
            .map_err(ClientError::http_error)?
            .error_for_status()
            .map_err(ClientError::http_error)?;
        Ok(())
    }
}
