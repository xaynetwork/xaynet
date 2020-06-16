use super::{
    error::Error,
    idle::Idle,
    requests::Sum2Request,
    CoordinatorState,
    Request,
    State,
    StateError,
    StateMachine,
};

use crate::{
    coordinator::RoundFailed,
    mask::{Aggregation, MaskObject, Model},
    PetError,
    SumParticipantPublicKey,
};
use std::{cmp::Ordering, mem};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Sum2;

impl State<Sum2> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
    ) -> StateMachine {
        info!("state transition");
        StateMachine::Sum2(Self {
            inner: Sum2,
            coordinator_state,
            request_rx,
        })
    }

    pub async fn next(mut self) -> StateMachine {
        match self.run_phase().await {
            Ok(_) => State::<Idle>::new(self.coordinator_state, self.request_rx),
            Err(err) => State::<Error>::new(self.coordinator_state, self.request_rx, err),
        }
    }

    async fn run_phase(&mut self) -> Result<(), StateError> {
        while !self.has_enough_sums() {
            let req = self.next_request().await?;
            self.handle_request(req);
        }
        let _model = self.end_round()?;
        Ok(())
    }

    /// Handle a sum, update or sum2 request.
    /// If the request is a sum or update request, the receiver of the response channel will receive
    /// a [`PetError::InvalidMessage`].
    fn handle_request(&mut self, req: Request) {
        match req {
            Request::Sum2(sum2_req) => self.handle_sum2(sum2_req),
            Request::Sum(sum_req) => Self::handle_invalid_message(sum_req.response_tx),
            Request::Update(update_req) => Self::handle_invalid_message(update_req.response_tx),
        }
    }

    /// Handle a sum2 request.
    fn handle_sum2(&mut self, req: Sum2Request) {
        let Sum2Request {
            participant_pk,
            mask,
            response_tx,
        } = req;

        // See `Self::handle_invalid_message`
        let _ = response_tx.send(self.add_mask(&participant_pk, mask));
    }

    /// Add a mask to the mask dictionary. Fails if the sum participant didn't register in the sum
    /// phase or it is a repetition.
    fn add_mask(&mut self, pk: &SumParticipantPublicKey, mask: MaskObject) -> Result<(), PetError> {
        // We move the participant key here to make sure a participant
        // cannot submit a mask multiple times
        if self.coordinator_state.sum_dict.remove(pk).is_none() {
            return Err(PetError::InvalidMessage);
        }

        if let Some(count) = self.coordinator_state.mask_dict.get_mut(&mask) {
            *count += 1;
        } else {
            self.coordinator_state.mask_dict.insert(mask, 1);
        }

        Ok(())
    }

    /// Check whether enough sum participants submitted their masks to start the idle phase.
    fn has_enough_sums(&self) -> bool {
        let mask_count = self.coordinator_state.mask_dict.values().sum::<usize>();
        mask_count >= self.coordinator_state.min_sum
    }

    /// Freeze the mask dictionary.
    fn freeze_mask_dict(&mut self) -> Result<MaskObject, RoundFailed> {
        if self.coordinator_state.mask_dict.is_empty() {
            return Err(RoundFailed::NoMask);
        }

        self.coordinator_state
            .mask_dict
            .drain()
            .fold(
                (None, 0_usize),
                |(unique_mask, unique_count), (mask, count)| match unique_count.cmp(&count) {
                    Ordering::Less => (Some(mask), count),
                    Ordering::Greater => (unique_mask, unique_count),
                    Ordering::Equal => (None, unique_count),
                },
            )
            .0
            .ok_or(RoundFailed::AmbiguousMasks)
    }

    fn end_round(&mut self) -> Result<Model, RoundFailed> {
        let global_mask = self.freeze_mask_dict()?;

        let aggregation = mem::replace(
            &mut self.coordinator_state.aggregation,
            Aggregation::new(self.coordinator_state.mask_config),
        );

        aggregation
            .validate_unmasking(&global_mask)
            .map_err(RoundFailed::from)?;
        Ok(aggregation.unmask(global_mask))
    }
}
