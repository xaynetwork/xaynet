use super::{
    error::Error,
    requests::UpdateRequest,
    sum2::Sum2,
    CoordinatorState,
    Request,
    State,
    StateError,
    StateMachine,
};
use crate::{LocalSeedDict, PetError, UpdateParticipantPublicKey};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Update;

impl State<Update> {
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
    ) -> StateMachine {
        info!("state transition");
        StateMachine::Update(Self {
            _inner: Update,
            coordinator_state,
            request_rx,
        })
    }

    pub async fn next(mut self) -> StateMachine {
        match self.run_phase().await {
            Ok(_) => State::<Sum2>::new(self.coordinator_state, self.request_rx),
            Err(err) => State::<Error>::new(self.coordinator_state, self.request_rx, err),
        }
    }

    async fn run_phase(&mut self) -> Result<(), StateError> {
        while !self.has_enough_seeds() {
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
            response_tx,
        } = req;
        let _ = response_tx.send(self.add_local_seed_dict(&participant_pk, &local_seed_dict));
    }

    /// Add a local seed dictionary to the seed dictionary. Fails if
    /// it contains invalid keys or it is a repetition.
    fn add_local_seed_dict(
        &mut self,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> Result<(), PetError> {
        if local_seed_dict.keys().len() == self.coordinator_state.sum_dict.keys().len()
            && local_seed_dict
                .keys()
                .all(|pk| self.coordinator_state.sum_dict.contains_key(pk))
            && self
                .coordinator_state
                .seed_dict
                .values()
                .next()
                .map_or(true, |dict| !dict.contains_key(pk))
        {
            for (sum_pk, seed) in local_seed_dict {
                self.coordinator_state
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
    fn has_enough_seeds(&self) -> bool {
        self.coordinator_state
            .seed_dict
            .values()
            .next()
            .map(|dict| dict.len() >= self.coordinator_state.min_update)
            .unwrap_or(false)
    }
}
