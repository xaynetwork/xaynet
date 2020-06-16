use super::{
    requests::Sum2Request,
    unmask::Unmask,
    CoordinatorState,
    MaskDict,
    PhaseState,
    Request,
    StateError,
    StateMachine,
    SumDict,
};

use crate::{
    mask::{Aggregation, MaskObject},
    PetError,
    SumParticipantPublicKey,
};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Sum2 {
    /// Dictionary built during the sum phase.
    sum_dict: SumDict,

    aggregation: Aggregation,

    /// Dictionary built during the sum2 phase.
    mask_dict: MaskDict,
}

impl PhaseState<Sum2> {
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: mpsc::UnboundedReceiver<Request>,
        sum_dict: SumDict,
        aggregation: Aggregation,
    ) -> Self {
        info!("state transition");
        Self {
            inner: Sum2 {
                sum_dict,
                aggregation,
                mask_dict: MaskDict::new(),
            },
            coordinator_state,
            request_rx,
        }
    }

    pub async fn next(mut self) -> Option<StateMachine> {
        let next_state = match self.run_phase().await {
            Ok(_) => PhaseState::<Unmask>::new(
                self.coordinator_state,
                self.request_rx,
                self.inner.aggregation,
                self.inner.mask_dict,
            )
            .into(),
            Err(err) => {
                PhaseState::<StateError>::new(self.coordinator_state, self.request_rx, err).into()
            }
        };
        Some(next_state)
    }

    async fn run_phase(&mut self) -> Result<(), StateError> {
        while !self.has_enough_sums() {
            let req = self.next_request().await?;
            self.handle_request(req);
        }
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
        if self.inner.sum_dict.remove(pk).is_none() {
            return Err(PetError::InvalidMessage);
        }

        if let Some(count) = self.inner.mask_dict.get_mut(&mask) {
            *count += 1;
        } else {
            self.inner.mask_dict.insert(mask, 1);
        }

        Ok(())
    }

    /// Check whether enough sum participants submitted their masks to start the idle phase.
    fn has_enough_sums(&self) -> bool {
        let mask_count = self.inner.mask_dict.values().sum::<usize>();
        mask_count >= self.coordinator_state.min_sum
    }
}
