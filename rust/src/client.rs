use crate::{
    crypto::ByteObject,
    mask::model::Model,
    participant::{Participant, Task},
    sdk::api::PrimitiveModel,
    service::{Handle, SerializedGlobalModel},
    CoordinatorPublicKey,
    InitError,
    PetError,
    SumDict,
    UpdateSeedDict,
};
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio::time;

/// Client-side errors
#[derive(Debug, Error)]
pub enum ClientError {
    /// Error starting the underlying [`Participant`]
    #[error("failed to initialise participant: {0}")]
    ParticipantInitErr(InitError),

    /// Error from the underlying [`Participant`]
    #[error("error arising from participant")]
    ParticipantErr(PetError),

    /// Error deserialising service data
    #[error("failed to deserialise service data: {0}")]
    DeserialiseErr(bincode::Error),

    /// General client errors
    #[error("unexpected client error")]
    GeneralErr,
}

/// A client of the federated learning service
///
/// [`Client`] is responsible for communicating with the service, deserialising
/// its messages and delegating their processing to the underlying
/// [`Participant`].
pub struct Client {
    /// Handle to the federated learning [`Service`]
    handle: Handle,

    /// The underlying [`Participant`]
    pub(crate) participant: Participant,

    /// Interval to poll for service data
    interval: time::Interval,

    /// Coordinator public key
    coordinator_pk: CoordinatorPublicKey,
    pub(crate) has_new_coord_pk_since_last_check: bool,

    pub(crate) global_model: Option<SerializedGlobalModel>,
    pub(crate) cached_model: Option<PrimitiveModel>,
    pub(crate) has_new_global_model_since_last_check: bool,
    pub(crate) has_new_global_model_since_last_cache: bool,
    pub(crate) local_model: Option<Model>,

    id: u32, // NOTE identifier for client for testing; may remove later
}

impl Client {
    /// Create a new [`Client`]
    ///
    /// `period`: time period at which to poll for service data, in seconds.
    /// Returns `Ok(client)` if [`Client`] `client` initialised successfully
    /// Returns `Err(err)` if `ClientError` `err` occurred
    pub fn new(period: u64) -> Result<Self, ClientError> {
        let (handle, _events) = Handle::new();
        let participant = Participant::new().map_err(ClientError::ParticipantInitErr)?;
        Ok(Self {
            handle,
            participant,
            interval: time::interval(Duration::from_secs(period)),
            coordinator_pk: CoordinatorPublicKey::zeroed(),
            has_new_coord_pk_since_last_check: false,
            global_model: None,
            cached_model: None,
            has_new_global_model_since_last_check: false,
            has_new_global_model_since_last_cache: false,
            local_model: None,
            id: 0,
        })
    }

    /// Create a new [`Client`] with ID (useful for testing)
    ///
    /// `period`: time period at which to poll for service data, in seconds.
    /// `id`: an ID to assign to the [`Client`].
    /// Returns `Ok(client)` if [`Client`] `client` initialised successfully
    /// Returns `Err(err)` if `ClientError` `err` occurred
    pub fn new_with_id(period: u64, handle: Handle, id: u32) -> Result<Self, ClientError> {
        let participant = Participant::new().map_err(ClientError::ParticipantInitErr)?;
        Ok(Self {
            handle,
            participant,
            interval: time::interval(Duration::from_secs(period)),
            coordinator_pk: CoordinatorPublicKey::zeroed(),
            has_new_coord_pk_since_last_check: false,
            global_model: None,
            cached_model: None,
            has_new_global_model_since_last_check: false,
            has_new_global_model_since_last_cache: false,
            local_model: None,
            id,
        })
    }

    /// Start the [`Client`] loop
    ///
    /// Returns `Err(err)` if `ClientError` `err` occurred
    /// NOTE in the future this may iterate only a fixed number of times, at the
    /// end of which this returns `Ok(())`
    pub async fn start(&mut self) -> Result<(), ClientError> {
        loop {
            // any error that bubbles up will finish off the client
            self.during_round().await?;
        }
    }

    /// [`Client`] duties within a round
    pub async fn during_round(&mut self) -> Result<Task, ClientError> {
        loop {
            if let Some(round_params_data) = self.handle.get_round_parameters().await {
                // new global model at the end of the current round
                if let Some(ref new_global_model) = round_params_data.global_model {
                    if let Some(ref old_global_model) = self.global_model {
                        if !Arc::ptr_eq(new_global_model, old_global_model) {
                            self.global_model = Some(new_global_model.clone());
                            self.has_new_global_model_since_last_check = true;
                            self.has_new_global_model_since_last_cache = true;
                        }
                    } else {
                        self.global_model = Some(new_global_model.clone());
                        self.has_new_global_model_since_last_check = true;
                        self.has_new_global_model_since_last_cache = true;
                    }
                }
                // new round parameters at the beginning of the next round
                if let Some(ref round_params) = round_params_data.round_parameters {
                    if round_params.pk != self.coordinator_pk {
                        // new round: save coordinator pk
                        self.coordinator_pk = round_params.pk;
                        self.has_new_coord_pk_since_last_check = true;
                        debug!(client_id = %self.id, "computing sigs and checking task");
                        let round_seed = round_params.seed.as_slice();
                        self.participant.compute_signatures(round_seed);
                        let (sum_frac, upd_frac) = (round_params.sum, round_params.update);
                        // perform duties as per my role for this round
                        break match self.participant.check_task(sum_frac, upd_frac) {
                            Task::Sum => self.summer().await,
                            Task::Update => self.updater().await,
                            Task::None => self.unselected().await,
                        };
                    }
                }
            }
            debug!(client_id = %self.id, "new round params not ready, retrying.");
            self.interval.tick().await;
        }
    }

    /// Duties for unselected [`Client`]s
    async fn unselected(&mut self) -> Result<Task, ClientError> {
        info!(client_id = %self.id, "not selected for this round");
        Ok(Task::None)
    }

    /// Duties for [`Client`]s selected as summers
    async fn summer(&mut self) -> Result<Task, ClientError> {
        info!(client_id = %self.id, "selected for sum, sending sum message.");
        let sum1_msg: Vec<u8> = self.participant.compose_sum_message(&self.coordinator_pk);
        self.handle.send_message(sum1_msg).await;

        let pk = self.participant.pk;
        let seed_dict_ser: Arc<Vec<u8>> = loop {
            if let Some(seed_dict_ser) = self.handle.get_seed_dict(pk).await {
                break seed_dict_ser;
            }
            debug!(client_id = %self.id, "seed dictionary not ready, retrying.");
            // updates not yet ready, try again later...
            self.interval.tick().await;
        };
        debug!(client_id = %self.id, "seed dictionary received");
        let seed_dict: UpdateSeedDict = bincode::deserialize(&seed_dict_ser[..]).map_err(|e| {
            error!(
                "failed to deserialize seed dictionary: {}: {:?}",
                e,
                &seed_dict_ser[..],
            );
            ClientError::DeserialiseErr(e)
        })?;
        debug!(client_id = %self.id, "sending sum2 message");
        let sum2_msg: Vec<u8> = self
            .participant
            .compose_sum2_message(self.coordinator_pk, &seed_dict)
            .map_err(ClientError::ParticipantErr)?;
        self.handle.send_message(sum2_msg).await;

        info!(client_id = %self.id, "sum participant completed a round");
        Ok(Task::Sum)
    }

    /// Duties for [`Client`]s selected as updaters
    async fn updater(&mut self) -> Result<Task, ClientError> {
        info!(client_id = %self.id, "selected to update");

        loop {
            if let Some(local_model) = self.local_model.take() {
                let (sum_dict_ser, scalar): (Arc<Vec<u8>>, f64) = loop {
                    if let Some((sum_dict_ser, scalar)) =
                        self.handle.get_sum_dict_and_scalar().await
                    {
                        break (sum_dict_ser, scalar);
                    }
                    debug!(client_id = %self.id, "sum dictionary not ready, retrying.");
                    // sums not yet ready, try again later...
                    self.interval.tick().await;
                };
                let sum_dict: SumDict = bincode::deserialize(&sum_dict_ser[..]).map_err(|e| {
                    error!(
                        "failed to deserialize sum dictionary: {}: {:?}",
                        e,
                        &sum_dict_ser[..],
                    );
                    ClientError::DeserialiseErr(e)
                })?;
                debug!(client_id = %self.id, "sum dictionary received, sending update message.");
                let upd_msg: Vec<u8> = self.participant.compose_update_message(
                    self.coordinator_pk,
                    &sum_dict,
                    scalar,
                    local_model,
                );
                self.handle.send_message(upd_msg).await;

                info!(client_id = %self.id, "update participant completed a round");
                break Ok(Task::Update);
            }
            debug!(client_id = %self.id, "local model not ready, retrying.");
            self.interval.tick().await;
        }
    }
}
