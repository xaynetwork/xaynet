use crate::{
    mask::{masking::Aggregation, object::MaskObject},
    state_machine::{
        coordinator::{CoordinatorState, MaskDict},
        events::PhaseEvent,
        phases::{Handler, Phase, PhaseState, StateError, Unmask},
        requests::{Request, RequestReceiver, Sum2Request, Sum2Response},
        StateMachine,
    },
    PetError,
    SumDict,
    SumParticipantPublicKey,
};

use tokio::sync::oneshot;

/// Sum2 state
#[derive(Debug)]
pub struct Sum2 {
    /// The sum dictionary built during the sum phase.
    sum_dict: SumDict,

    /// The aggregator for masks and masked models.
    aggregation: Aggregation,

    /// The mask dictionary built during the sum2 phase.
    mask_dict: MaskDict,
}

#[async_trait]
impl<R> Phase<R> for PhaseState<R, Sum2>
where
    Self: Handler<R>,
    R: Send,
{
    /// Moves from the sum2 state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn next(mut self) -> Option<StateMachine<R>> {
        info!("starting sum2 phase");

        info!("broadcasting sum2 phase event");
        self.coordinator_state
            .events
            .broadcast_phase(self.coordinator_state.round_params.id, PhaseEvent::Sum2);
        let next_state = match self.run_phase().await {
            Ok(_) => PhaseState::<R, Unmask>::new(
                self.coordinator_state,
                self.request_rx,
                self.inner.aggregation,
                self.inner.mask_dict,
            )
            .into(),
            Err(err) => {
                PhaseState::<R, StateError>::new(self.coordinator_state, self.request_rx, err)
                    .into()
            }
        };
        Some(next_state)
    }
}

impl<R> Handler<Request> for PhaseState<R, Sum2> {
    /// Handles a [`Request::Sum`], [`Request::Update`] or [`Request::Sum2`] request.
    ///
    /// If the request is a [`Request::Sum`] or [`Request::Update`] request, the request sender
    /// will receive a [`PetError::InvalidMessage`].
    fn handle_request(&mut self, req: Request) {
        match req {
            Request::Sum2((sum2_req, response_tx)) => self.handle_sum2(sum2_req, response_tx),
            Request::Sum((_, response_tx)) => Self::handle_invalid_message(response_tx),
            Request::Update((_, response_tx)) => Self::handle_invalid_message(response_tx),
        }
    }
}

impl<R> PhaseState<R, Sum2>
where
    Self: Handler<R>,
{
    /// Runs the sum2 phase.
    async fn run_phase(&mut self) -> Result<(), StateError> {
        while !self.has_enough_sums() {
            let req = self.next_request().await?;
            self.handle_request(req);
        }
        Ok(())
    }
}

impl<R> PhaseState<R, Sum2> {
    /// Creates a new sum2 state.
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: RequestReceiver<R>,
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

    /// Handles a sum2 request.
    /// If the handling of the sum2 message fails, an error is returned to the request sender.
    fn handle_sum2(&mut self, req: Sum2Request, response_tx: oneshot::Sender<Sum2Response>) {
        let Sum2Request {
            participant_pk,
            mask,
        } = req;

        // See `Self::handle_invalid_message`
        let _ = response_tx.send(self.add_mask(&participant_pk, mask));
    }

    /// Adds a mask to the mask dictionary.
    ///
    /// # Errors
    /// Fails if the sum participant didn't register in the sum phase or it is a repetition.
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

    /// Checks whether enough sum participants submitted their masks to start the idle phase.
    fn has_enough_sums(&self) -> bool {
        let mask_count = self.inner.mask_dict.values().sum::<usize>();
        mask_count >= self.coordinator_state.min_sum
    }
}
