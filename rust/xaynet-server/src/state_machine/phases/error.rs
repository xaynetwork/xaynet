use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;
use tokio::time::delay_for;
use tracing::{error, info};

use crate::{
    event,
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
    storage::{CoordinatorStorage, ModelStorage},
};

/// Error that can occur during the execution of the [`StateMachine`].
#[derive(Error, Debug)]
pub enum PhaseStateError {
    #[error("request channel error: {0}")]
    RequestChannel(&'static str),
    #[error("phase timeout")]
    PhaseTimeout(#[from] tokio::time::Elapsed),

    #[error("idle phase failed: {0}")]
    Idle(#[from] IdleStateError),

    #[error("sum phase failed: {0}")]
    Sum(#[from] SumStateError),

    #[error("update phase failed: {0}")]
    Update(#[from] UpdateStateError),

    #[error("unmask phase failed: {0}")]
    Unmask(#[from] UnmaskStateError),
}

impl<C, M> PhaseState<PhaseStateError, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Creates a new error phase.
    pub fn new(shared: Shared<C, M>, error: PhaseStateError) -> Self {
        Self {
            private: error,
            shared,
        }
    }

    /// Waits until the [`crate::storage::Store`] is ready.
    async fn wait_for_store_readiness(&mut self) {
        while let Err(err) = self.shared.store.is_ready().await {
            error!("store not ready: {}", err);
            info!("try again in 5 sec");
            delay_for(Duration::from_secs(5)).await;
        }
    }
}

#[async_trait]
impl<C, M> Phase<C, M> for PhaseState<PhaseStateError, C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    const NAME: PhaseName = PhaseName::Error;

    /// Runs the error phase.
    ///
    /// # Behavior
    ///
    /// The state machine remains in the error phase until the [`crate::storage::Store`] is ready.
    async fn run(&mut self) -> Result<(), PhaseStateError> {
        error!("phase state error: {}", self.private);

        event!("Phase error", &self.private.to_string());

        self.wait_for_store_readiness().await;

        Ok(())
    }

    /// Moves from the error phase to the next phase.
    ///
    /// See the [module level documentation](../index.html) for more details.
    fn next(self) -> Option<StateMachine<C, M>> {
        Some(match self.private {
            PhaseStateError::RequestChannel(_) => {
                PhaseState::<Shutdown, _, _>::new(self.shared).into()
            }
            _ => PhaseState::<Idle, _, _>::new(self.shared).into(),
        })
    }
}
