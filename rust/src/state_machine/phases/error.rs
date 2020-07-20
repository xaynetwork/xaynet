use crate::state_machine::{
    coordinator::CoordinatorState,
    events::PhaseEvent,
    phases::{Idle, Phase, PhaseState, Shutdown},
    requests::RequestReceiver,
    RoundFailed,
    StateMachine,
};
use thiserror::Error;

/// Error that can occur during the execution of the [`StateMachine`].
#[derive(Error, Debug)]
pub enum StateError {
    #[error("state failed: channel error: {0}")]
    ChannelError(&'static str),
    #[error("state failed: round error: {0}")]
    RoundError(#[from] RoundFailed),
}

impl<R> PhaseState<R, StateError> {
    /// Creates a new error state.
    pub fn new(
        coordinator_state: CoordinatorState,
        request_rx: RequestReceiver<R>,
        error: StateError,
    ) -> Self {
        info!("state transition");
        Self {
            inner: error,
            coordinator_state,
            request_rx,
        }
    }
}

#[async_trait]
impl<R> Phase<R> for PhaseState<R, StateError>
where
    R: Send,
{
    fn is_error(&self) -> bool {
        true
    }

    async fn run(&mut self) -> Result<(), StateError> {
        error!("state transition failed! error: {:?}", self.inner);

        info!("broadcasting error phase event");
        self.coordinator_state.events.broadcast_phase(
            self.coordinator_state.round_params.seed.clone(),
            PhaseEvent::Error,
        );

        Ok(())
    }

    /// Moves from the error state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    fn next(self) -> Option<StateMachine<R>> {
        Some(match self.inner {
            StateError::ChannelError(_) => {
                PhaseState::<R, Shutdown>::new(self.coordinator_state, self.request_rx).into()
            }
            _ => PhaseState::<R, Idle>::new(self.coordinator_state, self.request_rx).into(),
        })
    }
}
