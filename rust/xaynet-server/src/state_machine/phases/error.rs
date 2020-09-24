use crate::{
    state_machine::{
        phases::{Idle, Phase, PhaseName, PhaseState, Shared, Shutdown},
        RoundFailed,
        StateMachine,
    },
    storage::redis::RedisError,
};
use std::time::Duration;
use tokio::time::delay_for;

#[cfg(feature = "metrics")]
use crate::metrics;

use thiserror::Error;

/// Error that can occur during the execution of the [`StateMachine`].
#[derive(Error, Debug)]
pub enum StateError {
    #[error("state failed: channel error: {0}")]
    ChannelError(&'static str),
    #[error("state failed: round error: {0}")]
    RoundError(#[from] RoundFailed),
    #[error("state failed: phase timeout: {0}")]
    TimeoutError(#[from] tokio::time::Elapsed),
    #[error("state failed: Redis failed: {0}")]
    Redis(#[from] RedisError),
}

impl PhaseState<StateError> {
    /// Creates a new error state.
    pub fn new(shared: Shared, error: StateError) -> Self {
        info!("state transition");
        Self {
            inner: error,
            shared,
        }
    }
}

#[async_trait]
impl Phase for PhaseState<StateError> {
    const NAME: PhaseName = PhaseName::Error;

    async fn run(&mut self) -> Result<(), StateError> {
        error!("state transition failed! error: {:?}", self.inner);

        metrics!(
            self.shared.io.metrics_tx,
            metrics::phase::error::emit(&self.inner)
        );

        info!("broadcasting error phase event");
        self.shared.io.events.broadcast_phase(PhaseName::Error);

        if let StateError::Redis(_) = self.inner {
            // a simple loop that stops as soon as the redis client has reconnected to a redis
            // instance. Reconnecting a lost connection is handled internally by
            // redis::aio::ConnectionManager

            while self
                .shared
                .io
                .redis
                .connection()
                .await
                .ping()
                .await
                .is_err()
            {
                info!("try to reconnect to Redis in 5 sec");
                delay_for(Duration::from_secs(5)).await;
            }
        }

        Ok(())
    }

    /// Moves from the error state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    fn next(self) -> Option<StateMachine> {
        Some(match self.inner {
            StateError::ChannelError(_) => PhaseState::<Shutdown>::new(self.shared).into(),
            _ => PhaseState::<Idle>::new(self.shared).into(),
        })
    }
}
