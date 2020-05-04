use crate::service::Handle;
use crate::coordinator::RoundParameters;
use crate::participant::{Participant, Task};
use crate::{CoordinatorPublicKey, SeedDict, SumDict, PetError, InitError};
use sodiumoxide::crypto::box_;
use std::sync::Arc;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use tokio::time;
use std::time::Duration;
use thiserror::Error;


/// Client-side errors
#[derive(Debug, Error)]
pub enum ClientError {
    /// Error starting the underlying `Participant`
    #[error("failed to initialise participant: {0}")]
    ParticipantInitErr(InitError),

    /// Error from the underlying `Participant`
    #[error("error arising from participant")]
    ParticipantErr(PetError),

    /// Error deserialising service data
    #[error("failed to deserialise service data: {0}")]
    DeserialiseErr(bincode::Error),

    /// General client errors
    #[error("unexpected client error")]
    GeneralErr, // "Mastercard" error - may remove later
}

/// A client of the federated learning service
///
/// `Client` is responsible for communicating with the service, deserialising
/// its messages and delegating their processing to the underlying `Participant`.
pub struct Client {
    /// Handle to the federated learning `Service`
    handle: Handle,

    /// The underlying `Participant`
    participant: Participant,

    /// Interval to poll for service data
    interval: time::Interval,

    // TODO global model

    id: u32, // identifier for client; may remove later
}

impl Client {
    /// Create a new `Client`
    ///
    /// `period`: time period at which to poll for service data, in seconds.
    /// Returns `Ok(client)` if `Client` `client` initialised successfully
    /// Returns `Err(err)` if `ClientError` `err` occurred
    pub fn new(period: u64) -> Result<Self, ClientError> {
        let (handle, _events) = Handle::new();
        let participant = Participant::new()
            .map_err(ClientError::ParticipantInitErr)?;
        Ok(Self {
            handle,
            participant,
            interval: time::interval(Duration::from_secs(period)),
            id: 0,
        })
    }

    // may replace new later with this
    pub fn new2(period: u64, handle: Handle, id: u32) -> Result<Self, ClientError> {
        let participant = Participant::new()
            .map_err(ClientError::ParticipantInitErr)?;
        Ok(Self {
            handle,
            participant,
            interval: time::interval(Duration::from_secs(period)),
            id,
        })
    }

    /// Start the `Client` loop
    ///
    /// Returns `Err(err)` if `ClientError` `err` occurred
    /// NOTE in the future this may iterate only a fixed number of times, at the
    /// end of which this returns `Ok(())`
    pub async fn start(&mut self) -> Result<(), ClientError> {
        loop {
            // any error that bubbles up will finish off the client
            self.per_round().await?;
        }
    }

    /// Client duties within a round
    pub async fn per_round(&mut self) -> Result<Task, ClientError> {
        let round_params: Arc<RoundParameters> = loop {
            if let Some(round_params) =
                self.handle.get_round_parameters().await
            {
                break round_params
            }
            println!("{}: round params not ready yet, retrying in a sec...", self.id);
            self.interval.tick().await;
        };
        let round_seed: &[u8] = round_params.seed.as_slice();
        println!("computing sigs and checking task");
        self.participant
            .compute_signatures(round_seed);
        let (sum_frac, upd_frac) = (round_params.sum, round_params.update);
        match self
            .participant
            .check_task(sum_frac, upd_frac)
        {
            Task::Sum    =>
                self.summer(round_params.pk).await,
            Task::Update =>
                self.updater(round_params.pk).await,
            Task::None   =>
                self.unselected().await,
        }
    }

    /// Duties for unselected clients
    async fn unselected(&mut self) -> Result<Task, ClientError> {
        // TODO await global model; save it
        // end of round
        Ok(Task::None)
    }

    /// Duties for clients selected as summers
    async fn summer(&mut self, coord_pk: CoordinatorPublicKey)
                    -> Result<Task, ClientError>
    {
        let sum1_msg: Vec<u8> = self
            .participant
            .compose_sum_message(&coord_pk);
        self.handle
            .send_message(sum1_msg)
            .await;

        let pk = self.participant.get_encr_pk();
        let seed_dict_ser: Arc<Vec<u8>> = loop {
            if let Some(seed_dict_ser) = self.handle.get_seed_dict(pk).await {
                break seed_dict_ser
            }
            // updates not yet ready, try again later...
            self.interval.tick().await;
        };
        let seed_dict: SeedDict = bincode::deserialize(&seed_dict_ser[..])
            .map_err(ClientError::DeserialiseErr)?;
        let sum2_msg: Vec<u8> = self
            .participant
            .compose_sum2_message(&coord_pk, &seed_dict)
            .map_err(ClientError::ParticipantErr)?;
        self.handle
            .send_message(sum2_msg)
            .await;

        // job done, unselect
        //self.unselected()
        //    .await
        Ok(Task::Sum)
    }

    /// Duties for clients selected as updaters
    async fn updater(&mut self, coord_pk: CoordinatorPublicKey)
                     -> Result<Task, ClientError>
    {
        // TODO train a model update...
        let sum_dict_ser: Arc<Vec<u8>> = loop {
            if let Some(sum_dict_ser) = self.handle.get_sum_dict().await {
                break sum_dict_ser
            }
            // sums not yet ready, try again later...
            self.interval.tick().await;
        };
        let sum_dict: SumDict = bincode::deserialize(&sum_dict_ser[..])
            .map_err(ClientError::DeserialiseErr)?;
        let upd_msg: Vec<u8> = self
            .participant
            .compose_update_message(&coord_pk, &sum_dict);
        self.handle
            .send_message(upd_msg)
            .await;

        // job done, unselect
        //self.unselected()
        //    .await
        Ok(Task::Update)
    }
}










