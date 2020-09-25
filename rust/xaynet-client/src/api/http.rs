#[cfg(feature = "tls")]
use std::{
    fs,
    path::{Path, PathBuf},
};

use reqwest::{self, Client, ClientBuilder, Response, StatusCode};
use thiserror::Error;

#[cfg(feature = "tls")]
use reqwest::Certificate;

use crate::api::ApiClient;
use xaynet_core::{
    common::RoundParameters, crypto::ByteObject, mask::Model, SumDict, SumParticipantPublicKey,
    UpdateSeedDict,
};

#[derive(Debug)]
/// A client that communicates with the coordinator's API via HTTP(S).
pub struct HttpApiClient {
    /// HTTP client
    client: Client,
    /// Coordinator URL
    address: String,
}

impl HttpApiClient {
    /// Creates a new HTTP(S) client.
    ///
    /// Requires trusted server `certificates` if the `tls` feature is enabled.
    pub fn new<S>(
        address: S,
        #[cfg(feature = "tls")] certificates: Vec<Certificate>,
    ) -> Result<Self, HttpApiClientError>
    where
        S: Into<String>,
    {
        #[cfg(not(feature = "tls"))]
        let client = ClientBuilder::new()
            .build()
            .map_err(HttpApiClientError::Http)?;

        #[cfg(feature = "tls")]
        let client = if certificates.is_empty() {
            return Err(HttpApiClientError::NoCertificate);
        } else {
            let mut builder = ClientBuilder::new().use_rustls_tls();
            for certificate in certificates {
                builder = builder.add_root_certificate(certificate);
            }
            builder.build().map_err(HttpApiClientError::Http)?
        };

        Ok(Self {
            client,
            address: address.into(),
        })
    }

    #[cfg(feature = "tls")]
    /// Reads DER and PEM certificates from given paths.
    ///
    /// Requires the `tls` feature.
    pub fn certificates_from(paths: &[PathBuf]) -> Result<Vec<Certificate>, HttpApiClientError> {
        fn load_certificate(path: &Path) -> Result<Certificate, HttpApiClientError> {
            let encoding = fs::read(path).map_err(HttpApiClientError::Io)?;
            if let Some(extension) = path.extension() {
                match extension.to_str() {
                    Some("der") => {
                        Certificate::from_der(&encoding).map_err(HttpApiClientError::Http)
                    }
                    Some("pem") => {
                        Certificate::from_pem(&encoding).map_err(HttpApiClientError::Http)
                    }
                    _ => Err(HttpApiClientError::UnexpectedCertificate),
                }
            } else {
                Err(HttpApiClientError::UnexpectedCertificate)
            }
        }

        if paths.is_empty() {
            Err(HttpApiClientError::NoCertificate)
        } else {
            paths.iter().map(|path| load_certificate(path)).collect()
        }
    }
}

/// Error returned by an [`HttpApiClient`]
#[derive(Debug, Error)]
pub enum HttpApiClientError {
    #[error("failed to deserialize data: {0}")]
    Deserialize(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Unexpected response from the coordinator: {:?}", .0)]
    UnexpectedResponse(Response),

    #[error("Reading from file failed: {0}")]
    Io(#[from] std::io::Error),

    #[cfg(feature = "tls")]
    #[error("Unexpected certificate extension")]
    UnexpectedCertificate,

    #[cfg(feature = "tls")]
    #[error("No certificate found")]
    NoCertificate,
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
