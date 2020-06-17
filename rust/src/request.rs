use crate::{
    client::ClientError,
    crypto::ByteObject,
    request::Proxy::{InMem, Remote},
    service::{data::RoundParametersData, Handle},
    ParticipantPublicKey,
    SumDict,
    UpdateSeedDict,
};
use bytes::Bytes;
use reqwest::{Client, Error, IntoUrl, Response, StatusCode};

/// Proxy for communicating with the service.
pub enum Proxy {
    InMem(Handle),
    Remote(ClientReq),
}

impl Proxy {
    pub fn new(addr: &'static str) -> Self {
        Remote(ClientReq::new(addr))
    }

    pub async fn post_message(&self, msg: Vec<u8>) -> Result<(), ClientError> {
        match self {
            InMem(hdl) => {
                hdl.send_message(msg).await;
                Ok(())
            }
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

/// Manages client requests over HTTP
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
        self.simple_get(&url).await
    }

    async fn get_sums(&self) -> Result<Option<Bytes>, Error> {
        let url = format!("{}/sums", self.address);
        self.simple_get(&url).await
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

    async fn simple_get<T: IntoUrl>(&self, url: T) -> Result<Option<Bytes>, Error> {
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
