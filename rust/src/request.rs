use crate::{crypto::ByteObject, ParticipantPublicKey};
use crate::service::{Handle, data::RoundParametersData};
// use bytes::Bytes;
use reqwest::{Client, Error, StatusCode, Response};
// use reqwest::{Response, Url};
use crate::request::Proxy::{InMem, Remote};
use crate::client::ClientError;
use crate::{SumDict, UpdateSeedDict, SeedDict};
use bytes::Bytes;


/// Proxy for the client to communicate with the service.
pub enum Proxy {
    InMem(Handle),
    Remote(ClientReq),
}

impl Proxy {
    pub fn new(addr: &'static str) -> Self {
        Remote(ClientReq::new(addr))
    }

    // TODO post_message, get_sums etc.

    pub async fn post_message(&self, msg: Vec<u8>) -> Result<(), ClientError> {
        match self {
            InMem(hdl) => {
                hdl.send_message(msg).await;
                Ok(())
            },
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
            },
        }
    }

    pub async fn get_sums(&self) -> Result<Option<SumDict>, ClientError> {
        let opt_vec = match self {
            InMem(hdl) => {
                let opt_arc = hdl.get_sum_dict().await;
                opt_arc.map(|arc| (*arc).clone())
            },
            Remote(req) => {
                let opt_bytes = req.get_sums().await.map_err(|e| {
                    error!("failed to GET sum dict: {}", e);
                    ClientError::NetworkErr(e)
                })?;
                opt_bytes.map(|bytes| bytes.to_vec())
            },
        };
        let opt_sums = opt_vec.map(|vec| {
            bincode::deserialize(&vec[..]).map_err(|e| {
                error!("failed to deserialize sum dict: {}: {:?}", e, &vec[..]);
                ClientError::DeserialiseErr(e)
            })
        });
        opt_sums.transpose()
    }

    pub async fn get_seeds(&self, pk: ParticipantPublicKey) -> Result<Option<UpdateSeedDict>, ClientError> {
        let opt_vec = match self {
            InMem(hdl) => {
                let opt_arc = hdl.get_seed_dict(pk).await;
                opt_arc.map(|arc| (*arc).clone())
            },
            Remote(req) => {
                let opt_bytes = req._get_seeds(pk).await.map_err(|e| {
                    error!("failed to GET seed dict: {}", e);
                    ClientError::NetworkErr(e)
                })?;
                opt_bytes.map(|bytes| bytes.to_vec())
            },
        };
        let opt_seeds = opt_vec.map(|vec| {
            bincode::deserialize(&vec[..]).map_err(|e| {
                error!("failed to deserialize seed dict: {}: {:?}", e, &vec[..]);
                ClientError::DeserialiseErr(e)
            })
        });
        opt_seeds.transpose()
    }

    // pub async fn get_params(&self) -> Result<Option<RoundParametersData>, ClientError> {
    //     match self {
    //         InMem(hdl) => {
    //             hdl.get_
    //         },
    //         Remote(req) => {
    //         },
    //     };
    // }

}

impl From<Handle> for Proxy {
    fn from(hdl: Handle) -> Self {
        InMem(hdl)
    }
}


pub struct ClientReq {
    client: Client,
    address: &'static str,
}

impl ClientReq {
    pub fn new(address: &'static str) -> Self {
        Self {
            client: Client::new(),
            address,
        }
    }

    pub async fn _post_message(&self, msg: Vec<u8>) -> Result<StatusCode, Error> {
        let url = format!("{}/message", self.address);
        let response = self.client.post(&url).body(msg).send().await?;
        Ok(response.status())
    }

    pub async fn post_message(&self, msg: Vec<u8>) -> Result<Response, Error> {
        let url = format!("{}/message", self.address);
        let response = self.client.post(&url).body(msg).send().await?;
        response.error_for_status()
    }

    pub async fn _get_sums(&self) -> Result<Response, Error> {
        let url = format!("{}/sums", self.address);
        let response = self.client.get(&url).send().await?;
        response.error_for_status()
    }

    pub async fn get_sums(&self) -> Result<Option<Bytes>, Error> {
        let url = format!("{}/sums", self.address);
        let response = self.client.get(&url).send().await?;
        let good_resp = response.error_for_status()?;
        let opt_body = match good_resp.status() {
            StatusCode::NOT_FOUND => None,
            StatusCode::OK => Some(good_resp.bytes().await?),
            sc => {
                warn!("unexpected HTTP status code: {}", sc);
                None
            }
        };
        Ok(opt_body)
    }

    pub async fn _get_seeds(&self, pk: ParticipantPublicKey) -> Result<Option<Bytes>, Error> {
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
            StatusCode::NOT_FOUND => None,
            StatusCode::OK => Some(response.bytes().await?),
            sc => {
                warn!("unexpected HTTP status code: {}", sc);
                None
            }
        };
        Ok(opt_body)
    }

    pub async fn get_seeds(&self, pk: ParticipantPublicKey) -> Result<Vec<u8>, Error> {
        let url = format!("{}/seeds", self.address);
        let response = self
            .client
            .get(&url)
            .header("Content-Type", "application/octet-stream")
            .body(pk.as_slice().to_vec())
            .send()
            .await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    pub async fn get_params(&self) -> Result<Vec<u8>, Error> {
        let url = format!("{}/params", self.address);
        let response = self.client.get(&url).send().await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}

// pub async fn _get_seeds(&self, pk: ParticipantPublicKey) -> Result<Option<SeedDict>, ClientError> {
//     let opt_ser_seeds = match self {
//         InMem(hdl) => {
//             Ok(hdl.get_seed_dict(pk).await)
//         },
//         Remote(ser_seeds) => {
//             Err(ClientError::GeneralErr)
//         },
//     }?;
//     match opt_ser_seeds {
//         None => Ok(None),
//         Some(ser_seeds) => {
//             let seeds = bincode::deserialize(&ser_seeds[..]).map_err(|e| {
//                 error!("failed to deserialize seed dict: {}: {:?}", e, &ser_seeds[..]);
//                 ClientError::DeserialiseErr(e)
//             })?;
//             Ok(Some(seeds))
//         }
//     }
// }

