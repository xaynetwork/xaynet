use std::{fs, path::Path, sync::Arc};

use async_trait::async_trait;
use reqwest::{self, Certificate, ClientBuilder, Identity, Response, StatusCode};
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
    /// HTTP(S) client
    client: reqwest::Client,
    /// Coordinator URL
    address: Arc<str>,
}

impl Client {
    /// Creates a new HTTP(S) client.
    ///
    /// Optionally requires at least one of the following arguments to enable TLS:
    /// - `certificates`: DER or PEM encoded certificates to enable TLS server authentication.
    /// - `identity`: PEM encoded identity to enable TLS client authentication.
    ///
    /// # Errors
    /// Fails if the TLS settings are invalid.
    pub fn new<C, I>(address: &str, certificates: C, identity: I) -> Result<Self, ClientError>
    where
        C: Into<Option<Vec<Certificate>>>,
        I: Into<Option<Identity>>,
    {
        let address: Arc<str> = From::from(address);
        let certificates = certificates.into();
        let identity = identity.into();

        // without TLS
        if certificates.is_none() && identity.is_none() {
            let client = ClientBuilder::new().build().map_err(ClientError::Http)?;
            return Ok(Self { client, address });
        }

        // with TLS
        let mut builder = ClientBuilder::new().use_rustls_tls();
        if let Some(certificates) = certificates {
            if certificates.is_empty() {
                return Err(ClientError::NoCertificate);
            }
            for certificate in certificates {
                builder = builder.add_root_certificate(certificate);
            }
        }
        if let Some(identity) = identity {
            builder = builder.identity(identity);
        }
        let client = builder.build().map_err(ClientError::Http)?;
        Ok(Self { client, address })
    }

    /// Reads DER and PEM encoded certificates from given paths.
    ///
    /// # Errors
    /// Fails if the paths are empty, if a path doesn't contain a DER/PEM file or if this file can't
    /// be read.
    pub fn certificates_from<P, S>(paths: S) -> Result<Vec<Certificate>, ClientError>
    where
        P: AsRef<Path>,
        S: AsRef<[P]>,
    {
        fn load_certificate<P>(path: P) -> Result<Certificate, ClientError>
        where
            P: AsRef<Path>,
        {
            match path.as_ref().extension().map(|ext| ext.to_str()) {
                Some(Some("der")) => {
                    let encoding = fs::read(path).map_err(ClientError::Io)?;
                    Certificate::from_der(&encoding).map_err(ClientError::Http)
                }
                Some(Some("pem")) => {
                    let encoding = fs::read(path).map_err(ClientError::Io)?;
                    Certificate::from_pem(&encoding).map_err(ClientError::Http)
                }
                _ => Err(ClientError::UnexpectedCertificate),
            }
        }

        if paths.as_ref().is_empty() {
            Err(ClientError::NoCertificate)
        } else {
            paths.as_ref().iter().map(load_certificate).collect()
        }
    }

    /// Reads a PEM encoded identity from a given path.
    ///
    /// # Errors
    /// Fails if the path doesn't contain a PEM file or if this file can't be read.
    pub fn identity_from<P>(path: P) -> Result<Identity, ClientError>
    where
        P: AsRef<Path>,
    {
        if let Some(Some("pem")) = path.as_ref().extension().map(|ext| ext.to_str()) {
            let encoding = fs::read(path).map_err(ClientError::Io)?;
            Identity::from_pem(&encoding).map_err(ClientError::Http)
        } else {
            Err(ClientError::UnexpectedCertificate)
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
