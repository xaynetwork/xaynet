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
impl<S> Phase<S> for PhaseState<PhaseStateError, S>
where
    S: Storage,
{
    const NAME: PhaseName = PhaseName::Error;

    async fn process(&mut self) -> Result<(), PhaseStateError> {
        error!("phase state error: {}", self.private);
        event!("Phase error", self.private.to_string());

        Ok(())
    }

    async fn broadcast(&mut self) -> Result<(), PhaseStateError> {
        info!("broadcasting invalidation of sum dictionary from previous round");
        self.shared
            .events
            .broadcast_sum_dict(DictionaryUpdate::Invalidate);

        info!("broadcasting invalidation of seed dictionary from previous round");
        self.shared
            .events
            .broadcast_seed_dict(DictionaryUpdate::Invalidate);

        Ok(())
    }

    async fn next(mut self) -> Option<StateMachine<S>> {
        self.wait_for_store_readiness().await;

        Some(match self.private {
            PhaseStateError::RequestChannel(_) => {
                PhaseState::<Shutdown, _>::new(self.shared).into()
            }
            _ => PhaseState::<Idle, _>::new(self.shared).into(),
        })
    }
}

impl<S> PhaseState<PhaseStateError, S>
where
    S: Storage,
{
    /// Creates a new error phase.
    pub fn new(shared: Shared<S>, error: PhaseStateError) -> Self {
        Self {
            private: error,
            shared,
        }
    }

    /// Waits until the [`Store`] is ready.
    ///
    /// [`Store`]: crate::storage::Store
    async fn wait_for_store_readiness(&mut self) {
        while let Err(err) = <S as Storage>::is_ready(&mut self.shared.store).await {
            error!("store not ready: {}", err);
            info!("try again in 5 sec");
            sleep(Duration::from_secs(5)).await;
        }
    }
}
