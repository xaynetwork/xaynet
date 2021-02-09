use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{error, info};

use crate::{
    event,
    state_machine::{
        events::DictionaryUpdate,
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
    storage::Storage,
};

/// Error that can occur during the execution of the [`StateMachine`].
#[derive(Error, Debug)]
pub enum PhaseStateError {
    #[error("request channel error: {0}")]
    RequestChannel(&'static str),
    #[error("phase timeout")]
    PhaseTimeout(#[from] tokio::time::error::Elapsed),

    #[error("idle phase failed: {0}")]
    Idle(#[from] IdleStateError),

    #[error("sum phase failed: {0}")]
    Sum(#[from] SumStateError),

    #[error("update phase failed: {0}")]
    Update(#[from] UpdateStateError),

    #[error("unmask phase failed: {0}")]
    Unmask(#[from] UnmaskStateError),
}

#[async_trait]
impl<T> Phase<T> for PhaseState<PhaseStateError, T>
where
    T: Storage,
{
    const NAME: PhaseName = PhaseName::Error;

    async fn process(&mut self) -> Result<(), PhaseStateError> {
        error!("phase state error: {}", self.private);
        event!("Phase error", self.private.to_string());

        Ok(())
    }

    fn broadcast(&mut self) {
        info!("broadcasting invalidation of sum dictionary");
        self.shared
            .events
            .broadcast_sum_dict(DictionaryUpdate::Invalidate);

        info!("broadcasting invalidation of seed dictionary");
        self.shared
            .events
            .broadcast_seed_dict(DictionaryUpdate::Invalidate);
    }

    async fn next(mut self) -> Option<StateMachine<T>> {
        self.wait_for_store_readiness().await;

        Some(match self.private {
            PhaseStateError::RequestChannel(_) => {
                PhaseState::<Shutdown, _>::new(self.shared).into()
            }
            _ => PhaseState::<Idle, _>::new(self.shared).into(),
        })
    }
}

impl<T> PhaseState<PhaseStateError, T> {
    /// Creates a new error phase.
    pub fn new(shared: Shared<T>, error: PhaseStateError) -> Self {
        Self {
            private: error,
            shared,
        }
    }
}

impl<T> PhaseState<PhaseStateError, T>
where
    T: Storage,
{
    /// Waits until the [`Store`] is ready.
    ///
    /// [`Store`]: crate::storage::Store
    async fn wait_for_store_readiness(&mut self) {
        while let Err(err) = <T as Storage>::is_ready(&mut self.shared.store).await {
            error!("store not ready: {}", err);
            info!("try again in 5 sec");
            sleep(Duration::from_secs(5)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;
    use crate::{
        state_machine::{events::DictionaryUpdate, tests::builder::StateMachineBuilder},
        storage::tests::init_store,
    };

    #[tokio::test]
    #[serial]
    async fn integration_error_to_shutdown() {
        let store = init_store().await;
        let (state_machine, _request_tx, events) = StateMachineBuilder::new(store.clone())
            .with_phase(PhaseStateError::RequestChannel(""))
            .build();
        assert!(state_machine.is_error());

        let state_machine = state_machine.next().await.unwrap();
        assert!(state_machine.is_shutdown());

        // Check all the events that should be emitted during the error phase
        assert_eq!(events.phase_listener().get_latest().event, PhaseName::Error);
        assert_eq!(
            events.sum_dict_listener().get_latest().event,
            DictionaryUpdate::Invalidate,
        );
        assert_eq!(
            events.seed_dict_listener().get_latest().event,
            DictionaryUpdate::Invalidate,
        );
    }
}
