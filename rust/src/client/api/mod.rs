//! Provides functionality to enable clients to communicate with a XayNet
//! service over HTTP.
//!
//! See the [client module] documentation since this is a private module anyways.
//!
//! [client module]: ../index.html

mod http;
mod in_memory;

pub use self::{
    http::{HttpApiClient, HttpApiClientError},
    in_memory::{InMemoryApiClient, InMemoryApiClientError},
};

use crate::{
    mask::model::Model,
    common::RoundParameters,
    SumDict,
    SumParticipantPublicKey,
    UpdateSeedDict,
};

#[async_trait]
pub trait ApiClient {
    type Error: ::std::fmt::Debug + ::std::error::Error + 'static;

    /// Retrieve the current round parameters
    async fn get_round_params(&mut self) -> Result<RoundParameters, Self::Error>;

    /// Retrieve the current sum dictionary, if available
    async fn get_sums(&mut self) -> Result<Option<SumDict>, Self::Error>;

    /// Retrieve the current seed dictionary for the given sum
    /// participant, if available.
    async fn get_seeds(
        &mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<Option<UpdateSeedDict>, Self::Error>;

    /// Retrieve the current model/mask length, if available
    async fn get_mask_length(&mut self) -> Result<Option<u64>, Self::Error>;

    /// Retrieve the current global model, if available.
    async fn get_model(&mut self) -> Result<Option<Model>, Self::Error>;

    async fn send_message(&mut self, msg: Vec<u8>) -> Result<(), Self::Error>;
}
