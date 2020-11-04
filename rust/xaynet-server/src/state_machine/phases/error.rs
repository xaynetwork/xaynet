use crate::{
    state_machine::{
        phases::{Idle, Phase, PhaseName, PhaseState, Shared, Shutdown},
        StateMachine, UnmaskGlobalModelError,
    },
    storage::api::Store,
};
use std::time::Duration;
use tokio::time::delay_for;

#[cfg(feature = "metrics")]
use crate::metrics;

use thiserror::Error;

/// Error that can occur during the execution of the [`StateMachine`].
#[derive(Error, Debug)]
pub enum PhaseStateError {
    #[error("channel error: {0}")]
    Channel(&'static str),
    #[error("unmask global model error: {0}")]
    UnmaskGlobalModel(#[from] UnmaskGlobalModelError),
    #[error("phase timeout")]
    Timeout(#[from] tokio::time::Elapsed),
    #[error("failed to update the coordinator state: {0}")]
    UpdateCoordinatorState(crate::storage::api::StorageError),
    #[error("failed to clear dictionaries: {0}")]
    ClearDictionaries(crate::storage::api::StorageError),
    #[error("failed to retrieve sum dictionary: {0}")]
    GetSumDict(crate::storage::api::StorageError),
    #[error("failed to retrieve seed dictionary: {0}")]
    GetSeedDict(crate::storage::api::StorageError),
    #[error("failed to retrieve masks: {0}")]
    GetMasks(crate::storage::api::StorageError),
    #[error("failed to save the global model: {0}")]
    SaveGlobalModel(crate::storage::api::StorageError),
}

impl<St> PhaseState<PhaseStateError, St>
where
    St: Store,
{
    /// Creates a new error state.
    pub fn new(shared: Shared<St>, error: PhaseStateError) -> Self {
        Self {
            inner: error,
            shared,
        }
    }
}

#[async_trait]
impl<St> Phase<St> for PhaseState<PhaseStateError, St>
where
    St: Store,
{
    const NAME: PhaseName = PhaseName::Error;

    async fn run(&mut self) -> Result<(), PhaseStateError> {
        error!("state failed: {}", self.inner);

        metrics!(
            self.shared.metrics_tx,
            metrics::phase::error::emit(&self.inner)
        );

        match self.inner {
            PhaseStateError::Channel(_) => {}
            _ => {
                // a simple loop that stops as soon as the redis client has reconnected to a redis
                // instance. Reconnecting a lost connection is handled internally by
                // redis::aio::ConnectionManager
                while self.shared.store.get_coordinator_state().await.is_err() {
                    info!("try to reconnect to Redis in 5 sec");
                    delay_for(Duration::from_secs(5)).await;
                }
            }
        };

        Ok(())
    }

    /// Moves from the error state to the next state.
    ///
    /// See the [module level documentation](../index.html) for more details.
    fn next(self) -> Option<StateMachine<St>> {
        Some(match self.inner {
            PhaseStateError::Channel(_) => PhaseState::<Shutdown, _>::new(self.shared).into(),
            _ => PhaseState::<Idle, _>::new(self.shared).into(),
        })
    }
}
