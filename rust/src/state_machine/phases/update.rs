use std::sync::Arc;

use crate::{
    mask::{masking::Aggregation, object::MaskObject},
    state_machine::{
        coordinator::CoordinatorState,
        events::{DictionaryUpdate, MaskLengthUpdate, PhaseEvent, ScalarUpdate},
        phases::{Handler, Phase, PhaseState, StateError, Sum2},
        requests::{Request, RequestReceiver, UpdateRequest, UpdateResponse},
        StateMachine,
    },
    LocalSeedDict,
    PetError,
    SeedDict,
    SumDict,
    UpdateParticipantPublicKey,
};

use tokio::sync::oneshot;

#[derive(Debug)]
pub struct Update {
    // The frozen sum dictionary of the sum phase.
    frozen_sum_dict: SumDict,
    /// Dictionary built during the update phase.
    seed_dict: SeedDict,
    /// The aggregated masked model being built in the current round.
    aggregation: Aggregation,
}

#[async_trait]
impl<R> Phase<R> for PhaseState<R, Update>
where
    Self: Handler<R>,
    R: Send,
{
    async fn next(mut self) -> Option<StateMachine<R>> {
        info!("starting update phase");

        info!("broadcasting update phase event");
        self.coordinator_state
            .events
            .broadcast_phase(self.coordinator_state.round_params.id, PhaseEvent::Update);

        let next_state = match self.run_phase().await {
            Ok(_) => {
                let PhaseState {
                    inner:
                        Update {
                            frozen_sum_dict,
                            seed_dict,
                            aggregation,
                        },
                    mut coordinator_state,
                    request_rx,
                } = self;

                info!("broadcasting mask length");
                coordinator_state.events.broadcast_mask_length(
                    coordinator_state.round_params.id,
                    MaskLengthUpdate::New(aggregation.len()),
                );

                info!("broadcasting the global seed dictionary");
                coordinator_state.events.broadcast_seed_dict(
                    coordinator_state.round_params.id,
                    DictionaryUpdate::New(Arc::new(seed_dict)),
                );

                PhaseState::<R, Sum2>::new(
                    coordinator_state,
                    request_rx,
                    frozen_sum_dict,
                    aggregation,
                )
                .into()
            }
            Err(err) => {
                PhaseState::<R, StateError>::new(self.coordinator_state, self.request_rx, err)
                    .into()
            }
        };
        Some(next_state)
    }
}

impl<R> Handler<Request> for PhaseState<R, Update> {
    fn handle_request(&mut self, req: Request) {
        match req {
            Request::Update((update_req, response_tx)) => {
                self.handle_update(update_req, response_tx)
            }
            Request::Sum((_, response_tx)) => Self::handle_invalid_message(response_tx),
            Request::Sum2((_, response_tx)) => Self::handle_invalid_message(response_tx),
        }
    }
}

impl<R> PhaseState<R, Update>
where
    Self: Handler<R>,
{
    async fn run_phase(&mut self) -> Result<(), StateError> {
        let scalar = 1_f64
            / (self.coordinator_state.expected_participants as f64
                * self.coordinator_state.round_params.update);
        info!("broadcasting scalar: {}", scalar);
        self.coordinator_state.events.broadcast_scalar(
            self.coordinator_state.round_params.id,
            ScalarUpdate::New(scalar),
        );

        loop {
            let updaters = self.updater_count();
            let min_updaters = self.coordinator_state.min_update;
            if updaters >= min_updaters {
                info!(
                    "ending update phase: {} updaters (expected at least {})",
                    updaters, min_updaters
                );
                return Ok(());
            }

            info!(
                "not enough updaters: {} updaters (expecting at least {})",
                updaters, min_updaters
            );
            info!("waiting for more update messages");
            let req = self.next_request().await?;
            self.handle_request(req);
        }
    }
}

impl<R> PhaseState<R, Update> {
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: RequestReceiver<R>,
        frozen_sum_dict: SumDict,
        seed_dict: SeedDict,
    ) -> Self {
        info!("state transition");
        Self {
            inner: Update {
                frozen_sum_dict,
                seed_dict,
                aggregation: Aggregation::new(coordinator_state.mask_config),
            },
            coordinator_state,
            request_rx,
        }
    }

    /// Handle a update request.
    fn handle_update(&mut self, req: UpdateRequest, response_tx: oneshot::Sender<UpdateResponse>) {
        let UpdateRequest {
            participant_pk,
            local_seed_dict,
            masked_model,
        } = req;

        // See `Self::handle_invalid_message`
        let _ = response_tx.send(self.update_seed_dict_and_aggregate_mask(
            &participant_pk,
            &local_seed_dict,
            masked_model,
        ));
    }

    /// Updates the local seed dict and aggregates the masked model.
    fn update_seed_dict_and_aggregate_mask(
        &mut self,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
        masked_model: MaskObject,
    ) -> Result<(), PetError> {
        // Check if aggregation can be performed. It is important to
        // do that _before_ updating the seed dictionary, because we
        // don't want to add the local seed dict if the corresponding
        // masked model is invalid
        debug!("checking whether the masked model can be aggregated");
        self.inner
            .aggregation
            .validate_aggregation(&masked_model)
            .map_err(|e| {
                warn!("aggregation error: {}", e);
                PetError::InvalidMessage
            })?;

        // Try to update local seed dict first. If this fail, we do
        // not want to aggregate the model.
        info!("updating the global seed dictionary");
        self.add_local_seed_dict(pk, local_seed_dict)
            .map_err(|err| {
                warn!("invalid local seed dictionary, ignoring update message");
                err
            })?;

        info!("aggregating the masked model");
        self.inner.aggregation.aggregate(masked_model);
        Ok(())
    }

    /// Add a local seed dictionary to the seed dictionary. Fails if
    /// it contains invalid keys or it is a repetition.
    fn add_local_seed_dict(
        &mut self,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> Result<(), PetError> {
        if local_seed_dict.keys().len() == self.inner.frozen_sum_dict.keys().len()
            && local_seed_dict
                .keys()
                .all(|pk| self.inner.frozen_sum_dict.contains_key(pk))
            && self
                .inner
                .seed_dict
                .values()
                .next()
                .map_or(true, |dict| !dict.contains_key(pk))
        {
            debug!("adding local seed dictionary");
            for (sum_pk, seed) in local_seed_dict {
                self.inner
                    .seed_dict
                    .get_mut(sum_pk)
                    .ok_or(PetError::InvalidMessage)?
                    .insert(*pk, seed.clone());
            }
            Ok(())
        } else {
            warn!("invalid seed dictionary");
            Err(PetError::InvalidMessage)
        }
    }

    /// Return the number of update participants that sent a valid
    /// update message
    fn updater_count(&self) -> usize {
        self.inner
            .seed_dict
            .values()
            .next()
            .map(|dict| dict.len())
            .unwrap_or(0)
    }
}
