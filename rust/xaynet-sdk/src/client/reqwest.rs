use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use reqwest::{self, Certificate, ClientBuilder, Response, StatusCode};
use thiserror::Error;

use crate::XaynetClient;
use xaynet_core::{
    common::RoundParameters,
    crypto::ByteObject,
    mask::Model,
    SumDict,
    SumParticipantPublicKey,
    UpdateSeedDict,
};

#[derive(Debug, Clone)]
/// A client that communicates with the coordinator's API via HTTP(S).
pub struct Client {
    /// HTTP client
    client: reqwest::Client,
    /// Coordinator URL
    address: Arc<String>,
}

impl Client {
    /// Creates a new HTTP(S) client.
    ///
    /// Requires trusted server `certificates` if the `tls` feature is enabled.
    pub fn new<S>(address: S, certificates: Option<Vec<Certificate>>) -> Result<Self, ClientError>
    where
        S: Into<String>,
    {
        let address = Arc::new(address.into());
        if certificates.is_none() {
            return Ok(Self {
                client: ClientBuilder::new().build().map_err(ClientError::Http)?,
                address,
            });
        }

        let certificates = certificates.unwrap();
        let client = if certificates.is_empty() {
            return Err(ClientError::NoCertificate);
        } else {
            let mut builder = ClientBuilder::new().use_rustls_tls();
            for certificate in certificates {
                builder = builder.add_root_certificate(certificate);
            }
            builder.build().map_err(ClientError::Http)?
        };

        Ok(Self { client, address })
    }

    /// Reads DER and PEM certificates from given paths.
    ///
    /// Requires the `tls` feature.
    pub fn certificates_from(paths: &[PathBuf]) -> Result<Vec<Certificate>, ClientError> {
        fn load_certificate(path: &Path) -> Result<Certificate, ClientError> {
            let encoding = fs::read(path).map_err(ClientError::Io)?;
            if let Some(extension) = path.extension() {
                match extension.to_str() {
                    Some("der") => Certificate::from_der(&encoding).map_err(ClientError::Http),
                    Some("pem") => Certificate::from_pem(&encoding).map_err(ClientError::Http),
                    _ => Err(ClientError::UnexpectedCertificate),
                }
            } else {
                Err(ClientError::UnexpectedCertificate)
            }
        }

        if paths.is_empty() {
            Err(ClientError::NoCertificate)
        } else {
            paths.iter().map(|path| load_certificate(path)).collect()
        }
    }
}

/// Error returned by an [`Client`]
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("failed to deserialize data: {0}")]
    Deserialize(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Unexpected response from the coordinator: {:?}", .0)]
    UnexpectedResponse(Response),

    #[error("Reading from file failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unexpected certificate extension")]
    UnexpectedCertificate,

    #[error("No certificate found")]
    NoCertificate,
}

impl From<bincode::Error> for ClientError {
    fn from(e: bincode::Error) -> Self {
        Self::Deserialize(format!("{:?}", e))
    }
}

impl From<::std::num::ParseIntError> for ClientError {
    fn from(e: ::std::num::ParseIntError) -> Self {
        Self::Deserialize(format!("{:?}", e))
    }
}

#[async_trait]
impl XaynetClient for Client {
    type Error = ClientError;

    async fn get_round_params(&mut self) -> Result<RoundParameters, Self::Error> {
        let url = format!("{}/params", self.address);
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        if let StatusCode::OK = resp.status() {
            let body = resp.bytes().await?; //
            Ok(bincode::deserialize(&body[..])?)
        } else {
            Err(ClientError::UnexpectedResponse(resp))
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
            _ => Err(ClientError::UnexpectedResponse(resp)),
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
            _ => Err(ClientError::UnexpectedResponse(resp)),
        }
    }

    async fn get_mask_length(&mut self) -> Result<Option<u64>, Self::Error> {
        let url = format!("{}/length", self.address);
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        match resp.status() {
            StatusCode::OK => Ok(Some(resp.text().await?.parse()?)),
            StatusCode::NO_CONTENT => Ok(None),
            _ => Err(ClientError::UnexpectedResponse(resp)),
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
            _ => Err(ClientError::UnexpectedResponse(resp)),
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
