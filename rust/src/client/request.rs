//! Provides functionality to enable clients to communicate with a XayNet
//! service over HTTP.
//!
//! This includes a [`Proxy`] which a [`Client`] can use to communicate with the
//! service. To summarise, the proxy:
//!
//! * Wraps either a [`Handle`] (for local comms) or a [`ClientReq`] (for remote
//!   comms over HTTP).
//! * In the latter case, deals with logging and wrapping of network errors.
//! * Deals with deserialization
//!
//! [`ClientReq`] is responsible for building the HTTP request and extracting
//! the response body. As an example:
//!
//! ```
//! async fn get_sums(&self) -> Result<Option<bytes::Bytes>, reqwest::Error>
//! ```
//!
//! issues a GET request for the sum dictionary. The return type reflects the
//! presence of networking `Error`s, but also the situation where the dictionary
//! is simply just not yet available on the service. That is, the type also
//! reflects the _optionality_ of the data availability.
//!
//! [`Proxy`] essentially takes this (deserializing the `Bytes` into a `SumDict`
//! while handling `Error`s into [`ClientError`]s) to expose the overall method
//!
//! ```
//! async fn get_sums(&self) -> Result<Option<SumDict>, ClientError>
//! ```

use crate::{
    client::{
        request::Proxy::{InMem, Remote},
        ClientError,
    },
    crypto::ByteObject,
    service::{data::RoundParametersData, Handle},
    ParticipantPublicKey,
    SumDict,
    UpdateSeedDict,
};
use bytes::Bytes;
use reqwest::{Client, Error, IntoUrl, Response, StatusCode};

#[derive(Debug)]
/// Proxy for communicating with the service.
pub enum Proxy {
    InMem(Handle),
    Remote(ClientReq),
}

impl Proxy {
    pub fn new(addr: &'static str) -> Self {
        Remote(ClientReq::new(addr))
    }

    /// Posts the given PET message to the service proxy.
    ///
    /// # Errors
    /// Returns `NetworkErr` if a network error occurs while posting the PET
    /// message.
    pub async fn post_message(&self, msg: Vec<u8>) -> Result<(), ClientError> {
        match self {
            InMem(hdl) => hdl.send_message(msg).await,
            Remote(req) => {
                let resp = req.post_message(msg).await.map_err(|e| {
                    error!("failed to POST message: {}", e);
                    ClientError::NetworkErr(e)
                })?;
                // erroring status codes already caught above
                let code = resp.status();
                if code != StatusCode::OK {
                    warn!("unexpected HTTP status code: {}", code)
                };
            }
        };
        Ok(())
    }

    /// Get the sum dictionary data from the service proxy.
    ///
    /// Returns `Ok(Some(data))` if the `data` is available on the
    /// service, `Ok(None)` if it is not.
    ///
    /// # Errors
    /// Returns `NetworkErr` if a network error occurs while getting the data.
    /// Returns `DeserialiseErr` if an error occurs while deserialising the
    /// response.
    pub async fn get_sums(&self) -> Result<Option<SumDict>, ClientError> {
        let opt_vec = match self {
            InMem(hdl) => {
                let opt_arc = hdl.get_sum_dict().await;
                opt_arc.map(|arc| (*arc).clone())
            }
            Remote(req) => {
                let opt_bytes = req.get_sums().await.map_err(|e| {
                    error!("failed to GET sum dict: {}", e);
                    ClientError::NetworkErr(e)
                })?;
                opt_bytes.map(|bytes| bytes.to_vec())
            }
        };
        let opt_sums = opt_vec.map(|vec| {
            bincode::deserialize(&vec[..]).map_err(|e| {
                error!("failed to deserialize sum dict: {}: {:?}", e, &vec[..]);
                ClientError::DeserialiseErr(e)
            })
        });
        opt_sums.transpose()
    }

    /// Get the model scalar data from the service proxy.
    ///
    /// Returns `Ok(Some(data))` if the `data` is available on the
    /// service, `Ok(None)` if it is not.
    ///
    /// # Errors
    /// Returns `NetworkErr` if a network error occurs while getting the data.
    /// Returns `ParseErr` if an error occurs while parsing the response.
    pub async fn get_scalar(&self) -> Result<Option<f64>, ClientError> {
        match self {
            InMem(hdl) => Ok(hdl.get_scalar().await),
            Remote(req) => {
                let opt_text = req.get_scalar().await.map_err(|e| {
                    error!("failed to GET model scalar: {}", e);
                    ClientError::NetworkErr(e)
                })?;
                opt_text
                    .map(|text| {
                        text.parse().map_err(|e| {
                            error!("failed to parse model scalar: {}: {:?}", e, text);
                            ClientError::ParseErr
                        })
                    })
                    .transpose()
            }
        }
    }

    /// Get the seed dictionary data from the service proxy.
    ///
    /// Returns `Ok(Some(data))` if the `data` is available on the
    /// service, `Ok(None)` if it is not.
    ///
    /// # Errors
    /// Returns `NetworkErr` if a network error occurs while getting the data.
    /// Returns `DeserialiseErr` if an error occurs while deserialising the
    /// response.
    pub async fn get_seeds(
        &self,
        pk: ParticipantPublicKey,
    ) -> Result<Option<UpdateSeedDict>, ClientError> {
        let opt_vec = match self {
            InMem(hdl) => {
                let opt_arc = hdl.get_seed_dict(pk).await;
                opt_arc.map(|arc| (*arc).clone())
            }
            Remote(req) => {
                let opt_bytes = req.get_seeds(pk).await.map_err(|e| {
                    error!("failed to GET seed dict: {}", e);
                    ClientError::NetworkErr(e)
                })?;
                opt_bytes.map(|bytes| bytes.to_vec())
            }
        };
        let opt_seeds = opt_vec.map(|vec| {
            bincode::deserialize(&vec[..]).map_err(|e| {
                error!("failed to deserialize seed dict: {}: {:?}", e, &vec[..]);
                ClientError::DeserialiseErr(e)
            })
        });
        opt_seeds.transpose()
    }

    /// Get the model/mask length data from the service proxy.
    ///
    /// Returns `Ok(Some(data))` if the `data` is available on the
    /// service, `Ok(None)` if it is not.
    ///
    /// # Errors
    /// Returns `NetworkErr` if a network error occurs while getting the data.
    /// Returns `ParseErr` if an error occurs while parsing the response.
    pub async fn get_length(&self) -> Result<Option<u64>, ClientError> {
        match self {
            InMem(hdl) => Ok(hdl.get_length().await),
            Remote(req) => {
                let opt_text = req.get_length().await.map_err(|e| {
                    error!("failed to GET model/mask length: {}", e);
                    ClientError::NetworkErr(e)
                })?;
                opt_text
                    .map(|text| {
                        text.parse().map_err(|e| {
                            error!("failed to parse model/mask length: {}: {:?}", e, text);
                            ClientError::ParseErr
                        })
                    })
                    .transpose()
            }
        }
    }

    /// Get the round parameters data from the service proxy.
    ///
    /// Returns `Ok(Some(data))` if the `data` is available on the
    /// service, `Ok(None)` if it is not.
    ///
    /// # Errors
    /// Returns `NetworkErr` if a network error occurs while getting the data.
    /// Returns `DeserialiseErr` if an error occurs while deserialising the
    /// response.
    pub async fn get_params(&self) -> Result<Option<RoundParametersData>, ClientError> {
        let opt_vec = match self {
            InMem(hdl) => {
                let opt_arc = hdl.get_round_parameters().await;
                opt_arc.map(|arc| (*arc).clone())
            }
            Remote(req) => {
                let opt_bytes = req.get_params().await.map_err(|e| {
                    error!("failed to GET round parameters: {}", e);
                    ClientError::NetworkErr(e)
                })?;
                opt_bytes.map(|bytes| bytes.to_vec())
            }
        };
        let opt_params = opt_vec.map(|vec| {
            bincode::deserialize(&vec[..]).map_err(|e| {
                error!("failed to deserialize round params: {}: {:?}", e, &vec[..]);
                ClientError::DeserialiseErr(e)
            })
        });
        opt_params.transpose()
    }
}

impl From<Handle> for Proxy {
    fn from(hdl: Handle) -> Self {
        InMem(hdl)
    }
}

#[derive(Debug)]
/// Manages client requests over HTTP.
pub struct ClientReq {
    client: Client,
    address: &'static str,
}

impl ClientReq {
    fn new(address: &'static str) -> Self {
        Self {
            client: Client::new(),
            address,
        }
    }

    async fn post_message(&self, msg: Vec<u8>) -> Result<Response, Error> {
        let url = format!("{}/message", self.address);
        let response = self.client.post(&url).body(msg).send().await?;
        response.error_for_status()
    }

    async fn get_params(&self) -> Result<Option<Bytes>, Error> {
        let url = format!("{}/params", self.address);
        self.simple_get_bytes(&url).await
    }

    async fn get_sums(&self) -> Result<Option<Bytes>, Error> {
        let url = format!("{}/sums", self.address);
        self.simple_get_bytes(&url).await
    }

    async fn get_scalar(&self) -> Result<Option<String>, Error> {
        let url = format!("{}/scalar", self.address);
        self.simple_get_text(&url).await
    }

    async fn get_seeds(&self, pk: ParticipantPublicKey) -> Result<Option<Bytes>, Error> {
        let url = format!("{}/seeds", self.address);
        // send pk along as body of GET request
        let response = self
            .client
            .get(&url)
            .header("Content-Type", "application/octet-stream")
            .body(pk.as_slice().to_vec())
            .send()
            .await?
            .error_for_status()?;
        let opt_body = match response.status() {
            StatusCode::NO_CONTENT => None,
            StatusCode::OK => Some(response.bytes().await?),
            sc => {
                warn!("unexpected HTTP status code: {}", sc);
                None
            }
        };
        Ok(opt_body)
    }

    async fn get_length(&self) -> Result<Option<String>, Error> {
        let url = format!("{}/length", self.address);
        self.simple_get_text(&url).await
    }

    async fn simple_get_text<T: IntoUrl>(&self, url: T) -> Result<Option<String>, Error> {
        let response = self.client.get(url).send().await?;
        let good_resp = response.error_for_status()?;
        let opt_body = match good_resp.status() {
            StatusCode::NO_CONTENT => None,
            StatusCode::OK => Some(good_resp.text().await?),
            sc => {
                warn!("unexpected HTTP status code: {}", sc);
                None
            }
        };
        Ok(opt_body)
    }

    async fn simple_get_bytes<T: IntoUrl>(&self, url: T) -> Result<Option<Bytes>, Error> {
        let response = self.client.get(url).send().await?;
        let good_resp = response.error_for_status()?;
        let opt_body = match good_resp.status() {
            StatusCode::NO_CONTENT => None,
            StatusCode::OK => Some(good_resp.bytes().await?),
            sc => {
                warn!("unexpected HTTP status code: {}", sc);
                None
            }
        };
        Ok(opt_body)
    }
}
