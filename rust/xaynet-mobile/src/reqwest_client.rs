use std::{fs::File, io::Read};
use thiserror::Error;
use xaynet_sdk::client::Client;

/// Error returned upon failing to instantiate a new [`xaynet_sdk::client::Client`]
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("failed to read trust anchor {0}: {1}")]
    TrustAnchor(String, String),
    #[error("failed to read client certificate {0}: {1}")]
    ClientCert(String, String),
    #[error("{0}")]
    Other(String),
}

impl ClientError {
    fn trust_anchor<E: ::std::error::Error>(path: String, e: E) -> Self {
        Self::TrustAnchor(path, format!("{}", e))
    }

    fn client_cert<E: ::std::error::Error>(path: String, e: E) -> Self {
        Self::ClientCert(path, format!("{}", e))
    }

    fn other<E: ::std::error::Error>(e: E) -> Self {
        Self::Other(format!("{}", e))
    }
}

/// Build a new [`xaynet_sdk::client::Client`]
///
/// # Args
///
/// - `address`: URL of the Xaynet coordinator to connect to
/// - `trust_anchor_path`: path the to root certificate for TLS server authentication. The
///   certificate must be PEM encoded.
/// - `client_cert_path`: path to the client certificate to use for TLS client authentication. The
///   certificate must be PEM encoded.
pub fn new_client(
    address: &str,
    trust_anchor_path: Option<String>,
    client_cert_path: Option<String>,
) -> Result<Client<reqwest::Client>, ClientError> {
    let builder = reqwest::ClientBuilder::new();

    let builder = if let Some(path) = trust_anchor_path {
        let mut buf = Vec::new();
        File::open(&path)
            .map_err(|e| ClientError::trust_anchor(path.clone(), e))?
            .read_to_end(&mut buf)
            .map_err(|e| ClientError::trust_anchor(path.clone(), e))?;
        let root_cert =
            reqwest::Certificate::from_pem(&buf).map_err(|e| ClientError::trust_anchor(path, e))?;
        builder.use_rustls_tls().add_root_certificate(root_cert)
    } else {
        builder
    };

    let builder = if let Some(path) = client_cert_path {
        let mut buf = Vec::new();
        File::open(&path)
            .map_err(|e| ClientError::client_cert(path.clone(), e))?
            .read_to_end(&mut buf)
            .map_err(|e| ClientError::client_cert(path.clone(), e))?;
        let identity =
            reqwest::Identity::from_pem(&buf).map_err(|e| ClientError::client_cert(path, e))?;
        builder.use_rustls_tls().identity(identity)
    } else {
        builder
    };

    let reqwest_client = builder.build().map_err(ClientError::other)?;

    let xaynet_client = Client::new(reqwest_client, address)
        .map_err(|_| ClientError::InvalidUrl(address.to_string()))?;
    Ok(xaynet_client)
}
