use std::sync::Arc;

use crate::{
    state_machine::{
        coordinator::CoordinatorState,
        events::{DictionaryUpdate, PhaseEvent},
        phases::{Handler, Phase, PhaseState, StateError, Update},
        requests::{Request, RequestReceiver, SumRequest, SumResponse},
        StateMachine,
    },
    LocalSeedDict,
    SeedDict,
    SumDict,
};

use tokio::sync::oneshot;

/// Sum state
#[derive(Debug)]
pub struct Sum {
    /// Dictionary built during the sum phase.
    sum_dict: SumDict,
}

impl<R> Handler<Request> for PhaseState<R, Sum> {
    /// Handles a [`Request::Sum`], [`Request::Update`] or [`Request::Sum2`] request.\
    ///
    /// If the request is a [`Request::Update`] or [`Request::Sum2`] request, the request sender
    /// will receive a [`PetError::InvalidMessage`].
    ///
    /// [`PetError::InvalidMessage`]: crate::PetError::InvalidMessage
    fn handle_request(&mut self, req: Request) {
        match req {
            Request::Sum((sum_req, response_tx)) => self.handle_sum(sum_req, response_tx),
            Request::Update((_, response_tx)) => Self::handle_invalid_message(response_tx),
            Request::Sum2((_, response_tx)) => Self::handle_invalid_message(response_tx),
        }
    }
}

#[async_trait]
impl<R> Phase<R> for PhaseState<R, Sum>
where
    Self: Handler<R>,
    R: Send,
{
    /// Moves from the sum state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    async fn next(mut self) -> Option<StateMachine<R>> {
        info!("starting sum phase");

        info!("broadcasting sum phase event");
        self.coordinator_state
            .events
            .broadcast_phase(self.coordinator_state.round_params.id, PhaseEvent::Sum);
        let next_state = match self.run_phase().await {
            Ok(seed_dict) => PhaseState::<R, Update>::new(
                self.coordinator_state,
                self.request_rx,
                self.inner.sum_dict,
                seed_dict,
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

impl<R> PhaseState<R, Sum>
where
    Self: Handler<R>,
{
    /// Runs the sum phase.
    pub async fn run_phase(&mut self) -> Result<SeedDict, StateError> {
        while !self.has_enough_sums() {
            let req = self.next_request().await?;
            self.handle_request(req);
        }
        Ok(self.freeze_sum_dict())
    }
}

impl<R> PhaseState<R, Sum> {
    /// Creates a new sum state.
    pub fn new(coordinator_state: CoordinatorState, request_rx: RequestReceiver<R>) -> Self {
        info!("state transition");
        Self {
            inner: Sum {
                sum_dict: SumDict::new(),
            },
            coordinator_state,
            request_rx,
        }
    }

    /// Handles a sum request.
    fn handle_sum(&mut self, req: SumRequest, response_tx: oneshot::Sender<SumResponse>) {
        let SumRequest {
            participant_pk,
            ephm_pk,
        } = req;

        self.inner.sum_dict.insert(participant_pk, ephm_pk);

        // See `Self::handle_invalid_message`
        let _ = response_tx.send(Ok(()));
    }

    /// Freezes the sum dictionary.
    fn freeze_sum_dict(&mut self) -> SeedDict {
        info!("broadcasting sum dictionary");
        self.coordinator_state.events.broadcast_sum_dict(
            self.coordinator_state.round_params.id,
            DictionaryUpdate::New(Arc::new(self.inner.sum_dict.clone())),
        );

        info!("initializing seed dictionary");
        self.inner
            .sum_dict
            .keys()
            .map(|pk| (*pk, LocalSeedDict::new()))
            .collect()
    }

    /// Checks whether enough sum participants submitted their ephemeral keys to start the update
    /// phase.
    fn has_enough_sums(&self) -> bool {
        self.inner.sum_dict.len() >= self.coordinator_state.min_sum
    }
}
