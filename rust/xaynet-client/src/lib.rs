//! Provides client-side functionality to connect to a XayNet service.
//!
//! This functionality includes:
//!
//! * Abiding by (the underlying [`Participant`]'s side of) the PET protocol.
//! * Handling the network communication with the XayNet service, including
//!   polling of service data.
//!
//! # Participant
//! In any given round of federated learning, each [`Participant`] of the
//! protocol is characterised by a role which determines its [`Task`] to carry
//! out in the round, and which is computed by [`check_task`].
//!
//! Participants selected to `Update` are responsible for sending masked model
//! updates in the form of PET messages constructed with
//! [`compose_update_message`].
//!
//! Participants selected to `Sum` are responsible for sending ephemeral keys
//! and global masks in PET messages constructed respectively with
//! [`compose_sum_message`] and [`compose_sum2_message`].
//!
//! # Client
//! A [`Client`] has an intentionally simple API - the idea is that it is
//! initialised with some settings, and then [`start()`]ed. Currently for
//! simplicity, clients that have started running will do so indefinitely. It is
//! therefore the user's responsibility to terminate clients that are no longer
//! needed. Alternatively, it may be more convenient to run just a single round
//! (or a known fixed number of rounds). In this case, use [`during_round()`].
//! For examples of usage, see the `test-drive` scripts.
//!
//! **Note.** At present, the [`Client`] implementation is somewhat tightly
//! coupled with the workings of the C-API SDK, but this may well change in a
//! future version to be more independently reusable.
//!
//! [`check_task`]: #method.check_task
//! [`compose_update_message`]: #method.compose_update_message
//! [`compose_sum_message`]: #method.compose_sum_message
//! [`compose_sum2_message`]: #method.compose_sum2_message
//! [`start()`]: #method.start
//! [`during_round()`]: #method.during_round

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate tracing;

use std::time::Duration;

use thiserror::Error;
use tokio::time;

use xaynet_core::{crypto::ByteObject, mask::Model, CoordinatorPublicKey, InitError};

pub mod api;
#[doc(hidden)]
pub mod mobile_client;
mod participant;
pub use participant::{Participant, Task};
pub(crate) mod utils;

#[derive(Clone, Debug)]
/// A primitive model cached on the heap.
///
/// The fat pointer `PrimitiveModel` returned from `new_model()` and `get_model()` references
/// this memory.
pub enum CachedModel {
    F32(Vec<f32>),
    F64(Vec<f64>),
    I32(Vec<i32>),
    I64(Vec<i64>),
}

#[derive(Debug, Error)]
pub enum PetError {
    #[error("Invalid mask")]
    InvalidMask,
    #[error("Invalid model")]
    InvalidModel,
}

#[derive(Debug, Error)]
/// Client-side errors
pub enum ClientError<E: ::std::error::Error + ::std::fmt::Debug + 'static> {
    #[error("failed to initialise participant: {0}")]
    /// Failed to initialise participant.
    ParticipantInitErr(InitError),

    #[error("an API request failed: {0}")]
    /// Failed to handle PET message.
    Api(#[from] E),

    #[error("error arising from participant")]
    /// Error arising from participant.
    ParticipantErr(PetError),

    #[error("{0} not ready yet")]
    TooEarly(&'static str),

    #[error("round outdated")]
    RoundOutdated,
}

/// A client of the federated learning service
///
/// [`Client`] is responsible for communicating with the service, deserialising
/// its messages and delegating their processing to the underlying
/// [`Participant`].
pub struct Client<C: api::ApiClient> {
    /// The underlying [`Participant`]
    pub participant: Participant,

    /// Interval to poll for service data
    /// (this is a `Stream` of `Future`s which requires a runtime to create the `Client`)
    interval: time::Interval,

    /// Coordinator public key
    coordinator_pk: CoordinatorPublicKey,
    pub has_new_coord_pk_since_last_check: bool,

    pub global_model: Option<Model>,
    pub cached_model: Option<CachedModel>,
    pub has_new_global_model_since_last_check: bool,
    pub has_new_global_model_since_last_cache: bool,

    // TEMP pub visibility to allow access from test-drive
    pub local_model: Option<Model>,
    pub scalar: f64,

    /// Identifier for this client
    id: u32,

    /// Client for the services
    client: C,
}

impl<C> Client<C>
where
    C: api::ApiClient,
{
    /// Create a new [`Client`] with a given service address.
    ///
    /// * `period`: time period at which to poll for service data, in seconds.
    /// * `id`: an ID to assign to the [`Client`].
    /// * `addr`: service address to connect to.
    ///
    /// # Errors
    /// Returns a `ParticipantInitErr` if the underlying [`Participant`] is
    /// unable to initialize.
    pub fn new(period: u64, id: u32, api: C) -> Result<Self, ClientError<C::Error>> {
        Ok(Self {
            participant: Participant::new().map_err(ClientError::ParticipantInitErr)?,
            interval: time::interval(Duration::from_secs(period)),
            coordinator_pk: CoordinatorPublicKey::zeroed(),
            has_new_coord_pk_since_last_check: false,

            global_model: None,
            cached_model: None,
            has_new_global_model_since_last_check: false,
            has_new_global_model_since_last_cache: false,

            local_model: None,
            scalar: 1.0,

            id,
            client: api,
        })
    }

    /// Starts the [`Client`] loop, iterating indefinitely over each federated
    /// learning round.
    ///
    /// # Errors
    /// A [`ClientError`] may be returned when the round is not able to complete
    /// successfully.
    pub async fn start(&mut self) -> Result<(), ClientError<C::Error>> {
        loop {
            self.during_round().await?;
        }
    }

    /// [`Client`] work flow over a federated learning round. A successfully
    /// completed round will return the [`Task`] of the client.
    ///
    /// # Errors
    /// A [`ClientError`] may be returned when the round is not able to complete
    /// successfully.
    pub async fn during_round(&mut self) -> Result<Task, ClientError<C::Error>> {
        debug!(client_id = %self.id, "polling for new round parameters");
        loop {
            let model = self.client.get_model().await?;
            // update our global model where necessary
            match (model, &self.global_model) {
                (Some(new_model), None) => self.set_global_model(new_model),
                (Some(new_model), Some(old_model)) if &new_model != old_model => {
                    self.set_global_model(new_model)
                }
                (None, _) => trace!(client_id = %self.id, "global model not ready yet"),
                _ => trace!(client_id = %self.id, "global model still fresh"),
            }

            let round_params = self.client.get_round_params().await?;
            if round_params.pk != self.coordinator_pk {
                debug!(client_id = %self.id, "new round parameters received, determining task.");
                self.coordinator_pk = round_params.pk;
                let round_seed = round_params.seed.as_slice();
                self.participant.compute_signatures(round_seed);
                let (sum_frac, upd_frac) = (round_params.sum, round_params.update);

                // update the flag only after everthing else is done such that the client can learn
                // via the API that a new round has started once all parameters are available
                let task = self.participant.check_task(sum_frac, upd_frac);
                self.has_new_coord_pk_since_last_check = true;
                return match task {
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

    /// Work flow for unselected [`Client`]s.
    async fn unselected(&mut self) -> Result<Task, ClientError<C::Error>> {
        debug!(client_id = %self.id, "not selected");
        Ok(Task::None)
    }

    /// Work flow for [`Client`]s selected as sum participants.
    async fn summer(&mut self) -> Result<Task, ClientError<C::Error>> {
        info!(client_id = %self.id, "selected to sum");
        let msg = self.participant.compose_sum_message(self.coordinator_pk);
        let sealed_msg = self.participant.seal_message(&self.coordinator_pk, &msg);

        self.client.send_message(sealed_msg).await?;

        debug!(client_id = %self.id, "polling for model/mask length");
        let length = loop {
            if let Some(length) = self.client.get_mask_length().await? {
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
            if let Some(seeds) = self.client.get_seeds(self.participant.pk).await? {
                debug!(client_id = %self.id, "seed dict received, sending sum2 message.");
                let msg = self
                    .participant
                    .compose_sum2_message(self.coordinator_pk, &seeds, length)
                    .map_err(|e| {
                        error!("failed to compose sum2 message with seeds: {:?}", &seeds);
                        ClientError::ParticipantErr(e)
                    })?;
                let sealed_msg = self.participant.seal_message(&self.coordinator_pk, &msg);
                self.client.send_message(sealed_msg).await?;

                info!(client_id = %self.id, "sum participant completed a round");
                break Ok(Task::Sum);
            }
            trace!(client_id = %self.id, "seed dict not ready, retrying.");
            self.interval.tick().await;
        }
    }

    /// Work flow for [`Client`]s selected as update participants.
    async fn updater(&mut self) -> Result<Task, ClientError<C::Error>> {
        info!(client_id = %self.id, "selected to update");

        debug!(client_id = %self.id, "polling for local model");
        let model = loop {
            if let Some(model) = self.local_model.take() {
                self.local_model = Some(model.clone()); // TEMP needs to be removed later.
                                                        // it is required so that the clients run several rounds
                break model;
            }
            trace!(client_id = %self.id, "local model not ready, retrying.");
            self.interval.tick().await;
        };

        debug!(client_id = %self.id, "polling for sum dict");
        loop {
            if let Some(sums) = self.client.get_sums().await? {
                debug!(client_id = %self.id, "sum dict received, sending update message.");
                let msg = self.participant.compose_update_message(
                    self.coordinator_pk,
                    &sums,
                    self.scalar,
                    model,
                );
                let sealed_msg = self.participant.seal_message(&self.coordinator_pk, &msg);
                self.client.send_message(sealed_msg).await?;

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
