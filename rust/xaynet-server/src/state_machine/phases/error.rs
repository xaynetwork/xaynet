use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;
use tokio::time::delay_for;
use tracing::{error, info};

#[cfg(feature = "metrics")]
use crate::metrics;
use crate::{
    state_machine::{
        phases::{
            idle::IdleStateError,
            sum::SumStateError,
            unmask::UnmaskStateError,
            update::UpdateStateError,
            Idle,
            Phase,
            PhaseName,
            PhaseState,
            Shared,
            Shutdown,
        },
        StateMachine,
    },
    storage::CoordinatorStorage,
};
use xaynet_macros::metrics;

/// Error that can occur during the execution of the [`StateMachine`].
#[derive(Error, Debug)]
pub enum PhaseStateError {
    #[error("channel error: {0}")]
    Channel(&'static str),
    #[error("phase timeout")]
    Timeout(#[from] tokio::time::Elapsed),

    #[error("idle phase failed: {0}")]
    Idle(#[from] IdleStateError),

    #[error("sum phase failed: {0}")]
    Sum(#[from] SumStateError),

    #[error("update phase failed: {0}")]
    Update(#[from] UpdateStateError),

    #[error("unmask phase failed: {0}")]
    Unmask(#[from] UnmaskStateError),
}

impl PhaseState<PhaseStateError> {
    /// Creates a new error state.
    pub fn new(shared: Shared, error: PhaseStateError) -> Self {
        Self {
            inner: error,
            shared,
        }
    }
}

#[async_trait]
impl Phase for PhaseState<PhaseStateError> {
    const NAME: PhaseName = PhaseName::Error;

    async fn run(&mut self) -> Result<(), PhaseStateError> {
        error!("{}", self.inner);

        metrics!(
            self.shared.io.metrics_tx,
            metrics::phase::error::emit(&self.inner)
        );

        while self.shared.io.redis.coordinator_state().await.is_err() {
            info!("try to reconnect to Redis in 5 sec");
            delay_for(Duration::from_secs(5)).await;
        }

        Ok(())
    }

    /// Moves from the error state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    fn next(self) -> Option<StateMachine> {
        Some(match self.inner {
            PhaseStateError::Channel(_) => PhaseState::<Shutdown>::new(self.shared).into(),
            _ => PhaseState::<Idle>::new(self.shared).into(),
        })
    }
}
