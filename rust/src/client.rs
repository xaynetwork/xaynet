use crate::service::Handle;
use crate::participant::{Participant, Task};
use crate::{CoordinatorPublicKey, SeedDict, SumDict, PetError};
use sodiumoxide::crypto::box_;
use std::sync::Arc;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;


/// Client-side errors
pub enum ClientError {
    /// Error from the underlying `Participant`
    ParticipantErr(PetError),
    /// The round is not yet ready
    RoundNotReady,
    /// Not all sums are ready
    SumsNotReady,
    /// Not all updates are ready
    UpdatesNotReady,
}

/// A client of the federated learning service
pub struct Client {
    handle: Handle,
    particip: Participant,
    // coord_encr_pk: Option<box_::PublicKey>,
    // TODO global model
}

impl Client {

    /// Create a new `Client`
    pub fn new() -> Result<Self, ClientError> {
        let (handle, _events) = Handle::new();
        let particip = Participant::new()
            .map_err(ClientError::ParticipantErr)?;
        Ok(Self {
            handle,
            particip,
            // coord_encr_pk: None,
        })
    }

    /// Start the `Client`
    pub async fn start(&mut self) -> Result<(), ClientError> {
        loop {
            match self.per_round().await {
                Ok(()) => continue,
                // at the moment, any error finishes off the client
                error  => return error,
            }
        }
    }

    /// Client duties within a round
    async fn per_round(&mut self) -> Result<(), ClientError> {
        let round_params = self
            .handle
            .get_round_parameters()
            .await
            .ok_or(ClientError::RoundNotReady)?;
        let coord_pk = round_params.pk;
        let round_seed: &[u8] = round_params.seed.as_slice();
        self.particip
            .compute_signatures(round_seed);
        let (sum_frac, upd_frac) = (round_params.sum, round_params.update);
        match self.particip.check_task(sum_frac, upd_frac) {
            Task::Sum    =>
                self.summer(coord_pk).await,
            Task::Update =>
                self.updater(coord_pk).await,
            Task::None   =>
                self.unselected().await,
        }
    }

    /// Duties for unselected clients
    async fn unselected(&mut self) -> Result<(), ClientError> {
        // TODO await global model; save it
        // end of round
        Ok(())
    }

    /// Duties for clients selected as summers
    async fn summer(&mut self, coord_pk: CoordinatorPublicKey)
                    -> Result<(), ClientError> {
        let sum1_msg: Vec<u8> = self.particip
            .compose_sum_message(&coord_pk);
        self.handle
            .send_message(sum1_msg)
            .await;
        let _pk = self.particip.get_encr_pk();
        let _seed_dict: Arc<Vec<u8>> = self.handle
            .get_seed_dict() // later will need to pass pk
            .await
            .ok_or(ClientError::UpdatesNotReady)?;
        // TODO deserialize the seed_dict
        let dummy_seed_dict: SeedDict = HashMap::new(); // FIXME
        // https://github.com/servo/bincode
        // bincode::deserialize(&seed_dict[..]).unwrap()
        let sum2_msg: Vec<u8> = self.particip
            .compose_sum2_message(&coord_pk, &dummy_seed_dict)
            .map_err(ClientError::ParticipantErr)?;
        self.handle
            .send_message(sum2_msg)
            .await;

        // job done, unselect
        self.unselected()
            .await
    }

    /// Duties for clients selected as updaters
    async fn updater(&mut self, coord_pk: CoordinatorPublicKey)
                     -> Result<(), ClientError> {
        // TODO train a model update...
        let _sum_dict: Arc<Vec<u8>> = self
            .handle
            .get_sum_dict()
            .await
            .ok_or(ClientError::SumsNotReady)?;
        // TODO deserialise the sum dict
        let dummy_sum_dict: SumDict = HashMap::new(); // FIXME
        let upd_msg: Vec<u8> = self
            .particip
            .compose_update_message(&coord_pk, &dummy_sum_dict);
        self.handle
            .send_message(upd_msg)
            .await;

        // job done, unselect
        self.unselected()
            .await
    }
}

// TODO main
