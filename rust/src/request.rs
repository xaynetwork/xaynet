use crate::{crypto::ByteObject, ParticipantPublicKey};
use bytes::{Buf, Bytes};
use reqwest::{Client, Error, Response, StatusCode};

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

    pub async fn post_message(&self, msg: Vec<u8>) -> Result<StatusCode, Error> {
        let response = self.client.post(self.address).body(msg).send().await?;
        Ok(response.status())
    }

    pub async fn get_sums(&self) -> Result<Vec<u8>, Error> {
        // TODO append path
        let response = self.client.get(self.address).send().await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    pub async fn get_seeds(&self, pk: ParticipantPublicKey) -> Result<Vec<u8>, Error> {
        let response = self
            .client
            .get(self.address)
            .header("Content-Type", "application/octet-stream")
            .body(pk.as_slice().to_vec())
            .send()
            .await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    pub async fn get_params(&self) -> Result<Vec<u8>, Error> {
        let response = self.client.get(self.address).send().await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}
