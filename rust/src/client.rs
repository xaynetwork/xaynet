use std::time::Duration;

use thiserror::Error;
use tokio::time;

use crate::{
    crypto::ByteObject,
    mask::model::Model,
    participant::{Participant, Task},
    request::Proxy,
    sdk::api::CachedModel,
    services::{FetchError, PetMessageError},
    settings::MaskSettings,
    CoordinatorPublicKey,
    InitError,
    PetError,
};

#[derive(Debug, Error)]
/// Client-side errors
pub enum ClientError {
    #[error("failed to initialise participant: {0}")]
    ParticipantInitErr(InitError),

    #[error("Failed to retrieve data: {0}")]
    Fetch(FetchError),

    #[error("Failed to handle PET message: {0}")]
    PetMessage(PetMessageError),

    #[error("error arising from participant")]
    ParticipantErr(PetError),

    #[error("failed to deserialise service data: {0}")]
    DeserialiseErr(bincode::Error),

    #[error("network-related error: {0}")]
    NetworkErr(reqwest::Error),

    #[error("failed to parse service data")]
    ParseErr,

    #[error("unexpected client error")]
    GeneralErr,
}

/// A client of the federated learning service
///
/// [`Client`] is responsible for communicating with the service, deserialising
/// its messages and delegating their processing to the underlying
/// [`Participant`].
pub struct Client {
    /// The underlying [`Participant`]
    pub(crate) participant: Participant,

    /// Interval to poll for service data
    /// (this is a `Stream` of `Future`s which requires a runtime to create the `Client`)
    interval: time::Interval,

    /// Coordinator public key
    coordinator_pk: CoordinatorPublicKey,
    pub(crate) has_new_coord_pk_since_last_check: bool,

    pub(crate) global_model: Option<Model>,
    pub(crate) cached_model: Option<CachedModel>,
    pub(crate) has_new_global_model_since_last_check: bool,
    pub(crate) has_new_global_model_since_last_cache: bool,
    // TEMP pub visibility to allow access from test-drive
    pub local_model: Option<Model>,

    /// Identifier for this client
    id: u32,

    /// Proxy for the service
    proxy: Proxy,
}

impl Client {
    /// Create a new [`Client`] that connects to a default service address.
    ///
    /// `period`: time period at which to poll for service data, in seconds.
    ///
    /// Returns `Ok(client)` if [`Client`] `client` initialised successfully
    /// Returns `Err(err)` if `ClientError` `err` occurred
    pub fn new(settings: ClientSettings, proxy: Proxy) -> Result<Self, ClientError> {
        let participant =
            Participant::new(settings.mask_settings).map_err(ClientError::ParticipantInitErr)?;
        Ok(Self {
            participant,
            interval: time::interval(settings.period),
            coordinator_pk: CoordinatorPublicKey::zeroed(),
            has_new_coord_pk_since_last_check: false,
            global_model: None,
            cached_model: None,
            has_new_global_model_since_last_check: false,
            has_new_global_model_since_last_cache: false,
            local_model: None,
            id: settings.id,
            proxy,
        })
    }

    /// Start the [`Client`] loop
    ///
    /// Returns `Err(err)` if `ClientError` `err` occurred
    ///
    /// NOTE in the future this may iterate only a fixed number of times, at the
    /// end of which this returns `Ok(())`
    pub async fn start(&mut self) -> Result<(), ClientError> {
        loop {
            self.during_round().await?;
        }
    }

    /// [`Client`] duties within a round
    pub async fn during_round(&mut self) -> Result<Task, ClientError> {
        debug!(client_id = %self.id, "polling for new round parameters");
        loop {
            let model = self.proxy.get_model().await?;
            // update our global model where necessary
            match (model, &self.global_model) {
                (Some(new_model), None) => self.set_global_model(new_model),
                (Some(new_model), Some(old_model)) if &new_model != old_model => {
                    self.set_global_model(new_model)
                }
                (None, _) => trace!(client_id = %self.id, "global model not ready yet"),
                _ => trace!(client_id = %self.id, "global model still fresh"),
            }

            let round_params = self.proxy.get_round_params().await?;
            if round_params.pk != self.coordinator_pk {
                debug!(client_id = %self.id, "new round parameters received, determining task.");
                self.coordinator_pk = round_params.pk;
                self.has_new_coord_pk_since_last_check = true;
                let round_seed = round_params.seed.as_slice();
                self.participant.compute_signatures(round_seed);
                let (sum_frac, upd_frac) = (round_params.sum, round_params.update);

                return match self.participant.check_task(sum_frac, upd_frac) {
                    Task::Sum => self.summer().await,
                    Task::Update => self.updater().await,
                    Task::None => self.unselected().await,
                };
            } else {
                trace!(client_id = %self.id, "still the same round");
            }

            trace!(client_id = %self.id, "new round parameters not ready, retrying.");
            self.interval.tick().await;
        }
    }

    /// Duties for unselected [`Client`]s
    async fn unselected(&mut self) -> Result<Task, ClientError> {
        debug!(client_id = %self.id, "not selected");
        Ok(Task::None)
    }

    /// Duties for [`Client`]s selected as summers
    async fn summer(&mut self) -> Result<Task, ClientError> {
        info!(client_id = %self.id, "selected to sum");
        let sum1_msg = self.participant.compose_sum_message(&self.coordinator_pk);
        self.proxy.post_message(sum1_msg).await?;

        debug!(client_id = %self.id, "polling for model/mask length");
        let length = loop {
            if let Some(length) = self.proxy.get_mask_length().await? {
                if length > usize::MAX as u64 {
                    return Err(ClientError::ParticipantErr(PetError::InvalidModel));
                } else {
                    break length as usize;
                }
            }
            trace!(client_id = %self.id, "model/mask length not ready, retrying.");
            self.interval.tick().await;
        };

        debug!(client_id = %self.id, "sum message sent, polling for seed dict.");
        loop {
            if let Some(seeds) = self.proxy.get_seeds(self.participant.pk).await? {
                debug!(client_id = %self.id, "seed dict received, sending sum2 message.");
                let sum2_msg = self
                    .participant
                    .compose_sum2_message(self.coordinator_pk, &seeds, length)
                    .map_err(|e| {
                        error!("failed to compose sum2 message with seeds: {:?}", &seeds);
                        ClientError::ParticipantErr(e)
                    })?;
                self.proxy.post_message(sum2_msg).await?;

                info!(client_id = %self.id, "sum participant completed a round");
                break Ok(Task::Sum);
            }
            trace!(client_id = %self.id, "seed dict not ready, retrying.");
            self.interval.tick().await;
        }
    }

    /// Duties for [`Client`]s selected as updaters
    async fn updater(&mut self) -> Result<Task, ClientError> {
        info!(client_id = %self.id, "selected to update");

        debug!(client_id = %self.id, "polling for local model");
        let model = loop {
            if let Some(model) = self.local_model.take() {
                break model;
            }
            trace!(client_id = %self.id, "local model not ready, retrying.");
            self.interval.tick().await;
        };

        debug!(client_id = %self.id, "polling for model scalar");
        let scalar = loop {
            if let Some(scalar) = self.proxy.get_scalar().await? {
                break scalar;
            }
            trace!(client_id = %self.id, "model scalar not ready, retrying.");
            self.interval.tick().await;
        };

        debug!(client_id = %self.id, "polling for sum dict");
        loop {
            if let Some(sums) = self.proxy.get_sums().await? {
                debug!(client_id = %self.id, "sum dict received, sending update message.");
                let upd_msg = self.participant.compose_update_message(
                    self.coordinator_pk,
                    &sums,
                    scalar,
                    model,
                );
                self.proxy.post_message(upd_msg).await?;

                info!(client_id = %self.id, "update participant completed a round");
                break Ok(Task::Update);
            }
            trace!(client_id = %self.id, "sum dict not ready, retrying.");
            self.interval.tick().await;
        }
    }

    fn set_global_model(&mut self, model: Model) {
        debug!(client_id = %self.id, "updating global model");
        self.global_model = Some(model);
        self.has_new_global_model_since_last_check = true;
        self.has_new_global_model_since_last_cache = true;
    }
}

pub struct ClientSettings {
    id: u32,
    period: Duration,
    mask_settings: MaskSettings,
}

impl ClientSettings {
    pub fn new(id: u32, period_in_sec: u64, mask_settings: MaskSettings) -> Self {
        Self {
            id,
            period: Duration::from_secs(period_in_sec),
            mask_settings,
        }
    }
}
