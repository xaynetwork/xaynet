use super::{
    requests::UpdateRequest,
    sum2::Sum2,
    CoordinatorState,
    PhaseState,
    Request,
    SeedDict,
    StateError,
    StateMachine,
    SumDict,
};
use crate::{
    mask::{Aggregation, MaskObject},
    LocalSeedDict,
    PetError,
    UpdateParticipantPublicKey,
};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Update {
    // The frozen sum dictionary of the sum phase.
    frozen_sum_dict: SumDict,
    /// Dictionary built during the update phase.
    seed_dict: SeedDict,
    /// The aggregated masked model being built in the current round.
    aggregation: Aggregation,
}

impl PhaseState<Update> {
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
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

    pub async fn next(mut self) -> Option<StateMachine> {
        let next_state = match self.run_phase().await {
            Ok(_) => PhaseState::<Sum2>::new(
                self.coordinator_state,
                self.request_rx,
                self.inner.frozen_sum_dict,
                self.inner.aggregation,
            )
            .into(),
            Err(err) => {
                PhaseState::<StateError>::new(self.coordinator_state, self.request_rx, err).into()
            }
        };
        Some(next_state)
    }

    async fn run_phase(&mut self) -> Result<(), StateError> {
        while !self.has_enough_updates() {
            let req = self.next_request().await?;
            self.handle_request(req);
        }

        Ok(())
    }

    /// Handle a sum, update or sum2 request.
    /// If the request is a sum or sum2 request, the receiver of the response channel will receive
    /// a [`PetError::InvalidMessage`].
    fn handle_request(&mut self, req: Request) {
        match req {
            Request::Update(update_req) => self.handle_update(update_req),
            Request::Sum(sum_req) => Self::handle_invalid_message(sum_req.response_tx),
            Request::Sum2(sum2_req) => Self::handle_invalid_message(sum2_req.response_tx),
        }
    }

    /// Handle a update request.
    fn handle_update(&mut self, req: UpdateRequest) {
        let UpdateRequest {
            participant_pk,
            local_seed_dict,
            masked_model,
            response_tx,
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
        // Try to update local seed dict first. If this fail, we do
        // not want to aggregate the model.
        debug!("updating the global seed dictionary");
        self.add_local_seed_dict(pk, local_seed_dict)
            .map_err(|err| {
                warn!("invalid local seed dictionary, ignoring update message");
                err
            })?;

        // Check if aggregation can be performed, and do it.
        debug!("aggregating masked model");
        self.inner
            .aggregation
            .validate_aggregation(&masked_model)
            .map_err(|e| {
                warn!("aggregation error: {}", e);
                PetError::InvalidMessage
            })?;
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
            for (sum_pk, seed) in local_seed_dict {
                self.inner
                    .seed_dict
                    .get_mut(sum_pk)
                    .ok_or(PetError::InvalidMessage)?
                    .insert(*pk, seed.clone());
            }
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Check whether enough update participants submitted their models and seeds to start the sum2
    /// phase.
    fn has_enough_updates(&self) -> bool {
        self.inner
            .seed_dict
            .values()
            .next()
            .map(|dict| dict.len() >= self.coordinator_state.min_update)
            .unwrap_or(false)
    }
}
