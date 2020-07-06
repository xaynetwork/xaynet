//! Provides functionality to enable clients to communicate with a XayNet
//! service over HTTP.
//!
//! See the [client module] documentation since this is a private module anyways.
//!
//! [client module]: ../index.html

use crate::{
    client::{
        request::Proxy::{InMem, Remote},
        ClientError,
    },
    crypto::ByteObject,
    mask::Model,
    services::{Fetcher, PetMessageHandler},
    state_machine::coordinator::RoundParameters,
    SumDict,
    SumParticipantPublicKey,
    UpdateSeedDict,
};
use bytes::Bytes;
use reqwest::{Client, Error, IntoUrl, Response, StatusCode};

/// Proxy for communicating with the service.
pub enum Proxy {
    InMem(
        Box<dyn Fetcher + Send + Sync>,
        Box<dyn PetMessageHandler + Send + Sync>,
    ),
    Remote(ClientReq),
}

impl Proxy {
    /// TODO
    pub fn new_remote(addr: &'static str) -> Self {
        Remote(ClientReq::new(addr))
    }

    /// TODO
    pub fn new_in_mem(
        fetcher: impl Fetcher + 'static + Send + Sync,
        message_handler: impl PetMessageHandler + 'static + Send + Sync,
    ) -> Self {
        InMem(Box::new(fetcher), Box::new(message_handler))
    }

    /// Posts the given PET message to the service proxy.
    ///
    /// # Errors
    /// Returns `NetworkErr` if a network error occurs while posting the PET
    /// message.
    pub async fn post_message(&self, msg: Vec<u8>) -> Result<(), ClientError> {
        match self {
            InMem(_, hdl) => hdl
                .handle_message(msg)
                .await
                .map_err(ClientError::PetMessage),
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
                Ok(())
            }
        }
    }

    /// TODO
    pub async fn get_round_params(&self) -> Result<RoundParameters, ClientError> {
        match self {
            InMem(hdl, _) => hdl.round_params().await.map_err(ClientError::Fetch),
            Remote(req) => {
                let bytes = req.get_round_params().await.map_err(|e| {
                    error!("failed to GET round parameters: {}", e);
                    ClientError::NetworkErr(e)
                })?;
                bincode::deserialize(&bytes[..]).map_err(ClientError::DeserialiseErr)
            }
        }
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
        match self {
            InMem(hdl, _) => Ok(hdl
                                .sum_dict()
                                .await
                                .map_err(ClientError::Fetch)?
                                .map(|arc| (*arc).clone())),
            Remote(req) => {
                let bytes = req.get_sums().await.map_err(|e| {
                    error!("failed to GET sum dict: {}", e);
                    ClientError::NetworkErr(e)
                })?;
                if let Some(bytes) = bytes {
                    let sum_dict =
                        bincode::deserialize(&bytes[..]).map_err(ClientError::DeserialiseErr)?;
                    Ok(Some(sum_dict))
                } else {
                    Ok(None)
                }
            }
        }
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
            InMem(hdl, _) => hdl.scalar().await.map_err(ClientError::Fetch),
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
        pk: SumParticipantPublicKey,
    ) -> Result<Option<UpdateSeedDict>, ClientError> {
        match self {
            InMem(hdl, _) => Ok(hdl
                                .seed_dict()
                                .await
                                .map_err(ClientError::Fetch)?
                                .and_then(|dict| dict.get(&pk).cloned())),
            Remote(req) => req
                .get_seeds(pk)
                .await
                .map_err(|e| {
                    error!("failed to GET seed dict: {}", e);
                    ClientError::NetworkErr(e)
                })?
                .map(|bytes| {
                    let vec = bytes.to_vec();
                    bincode::deserialize(&vec[..]).map_err(|e| {
                        error!("failed to deserialize seed dict: {:?}", e);
                        ClientError::DeserialiseErr(e)
                    })
                })
                .transpose(),
        }
    }

    /// Get the model/mask length data from the service proxy.
    ///
    /// Returns `Ok(Some(data))` if the `data` is available on the
    /// service, `Ok(None)` if it is not.
    ///
    /// # Errors
    /// Returns `NetworkErr` if a network error occurs while getting the data.
    /// Returns `ParseErr` if an error occurs while parsing the response.
    pub async fn get_mask_length(&self) -> Result<Option<u64>, ClientError> {
        match self {
            // FIXME: don't cast here. The service just return an u64
            // not an usize
            InMem(hdl, _) => Ok(hdl
                                .mask_length()
                                .await
                                .map_err(ClientError::Fetch)?
                                .map(|len| len as u64)),
            Remote(req) => req
                .get_mask_length()
                .await
                .map_err(|e| {
                    error!("failed to GET model/mask length: {}", e);
                    ClientError::NetworkErr(e)
                })?
                .map(|text| {
                    text.parse().map_err(|e| {
                        error!("failed to parse model/mask length: {}: {:?}", e, text);
                        ClientError::ParseErr
                    })
                })
                .transpose(),
        }
    }

    /// FIXME Get the round parameters data from the service proxy.
    ///
    /// Returns `Ok(Some(data))` if the `data` is available on the
    /// service, `Ok(None)` if it is not.
    ///
    /// # Errors
    /// Returns `NetworkErr` if a network error occurs while getting the data.
    /// Returns `DeserialiseErr` if an error occurs while deserialising the
    /// response.
    pub async fn get_model(&self) -> Result<Option<Model>, ClientError> {
        match self {
            InMem(hdl, _) => Ok(hdl
                                .model()
                                .await
                                .map_err(ClientError::Fetch)?
                                .map(|arc| (*arc).clone())),
            Remote(req) => req
                .get_model()
                .await
                .map_err(|e| {
                    error!("failed to GET model: {}", e);
                    ClientError::NetworkErr(e)
                })?
                .map(|bytes| {
                    let vec = bytes.to_vec();
                    bincode::deserialize(&vec[..]).map_err(|e| {
                        error!("failed to deserialize model: {:?}", e);
                        ClientError::DeserialiseErr(e)
                    })
                })
                .transpose(),
        }
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

    async fn get_round_params(&self) -> Result<Bytes, Error> {
        let url = format!("{}/params", self.address);
        // FIXME don't unwrap
        Ok(self.simple_get_bytes(&url).await?.unwrap())
    }

    async fn get_sums(&self) -> Result<Option<Bytes>, Error> {
        let url = format!("{}/sums", self.address);
        self.simple_get_bytes(&url).await
    }

    async fn get_scalar(&self) -> Result<Option<String>, Error> {
        let url = format!("{}/scalar", self.address);
        self.simple_get_text(&url).await
    }

    async fn get_seeds(&self, pk: SumParticipantPublicKey) -> Result<Option<Bytes>, Error> {
        let url = format!("{}/seeds", self.address);
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

    async fn get_mask_length(&self) -> Result<Option<String>, Error> {
        let url = format!("{}/length", self.address);
        self.simple_get_text(&url).await
    }

    async fn get_model(&self) -> Result<Option<Bytes>, Error> {
        let url = format!("{}/model", self.address);
        self.simple_get_bytes(&url).await
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
