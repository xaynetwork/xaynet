use crate::{
    state_machine::{
        phases::{Idle, Phase, PhaseName, PhaseState, Shared},
        PhaseStateError, StateMachine,
    },
    storage::Storage,
};
use async_trait::async_trait;
use tracing::debug;
use tracing::error_span;
use tracing::info;
use tracing::warn;

/// Shutdown state
#[derive(Debug)]
pub struct Pause;

#[async_trait]
impl<S> Phase<S> for PhaseState<Pause, S>
where
    S: Storage,
{
    const NAME: PhaseName = PhaseName::Pause;

    async fn run(&mut self) -> Result<(), PhaseStateError> {
        self.shared.events.broadcast_readiness(true);
        Ok(())
    }

    fn next(self) -> Option<StateMachine<S>> {
        Some(PhaseState::<Idle, _>::new(self.shared).into())
    }
}

impl<S> PhaseState<Pause, S>
where
    S: Storage,
{
    /// Creates a new shutdown state.
    pub fn new(shared: Shared<S>) -> Self {
        Self {
            private: Pause,
            shared,
        }
    }
}

impl<T> PhaseState<Pause, T>
where
    T: Storage,
{
    /// Run the current phase to completion, then transition to the
    /// next phase and return it.
    pub async fn run_phase(mut self) -> Option<StateMachine<T>> {
        let phase = <Self as Phase<_>>::NAME;
        let span = error_span!("run_phase", phase = ?phase);

        async move {
            info!("starting phase");
            info!("broadcasting phase event");
            self.shared.events.broadcast_phase(phase);



            let delay = tokio::time::delay_for(tokio::time::Duration::from_secs(5));

            tokio::select! {
                _ =  delay => {
                    warn!("");
                }
                res = self.run() => {
                    if let Err(err) = res {
                        return Some(self.into_error_state(err));
                    }
                }
            }

            // if let Err(err) = self.run().await {
            //     return Some(self.into_error_state(err));
            // }

            info!("phase ran successfully");

            debug!("purging outdated requests before transitioning");
            if let Err(err) = self.purge_outdated_requests() {
                warn!("failed to purge outdated requests");
                // If we're already in the error state or shutdown state,
                // ignore this error
                match phase {
                    PhaseName::Error | PhaseName::Shutdown => {
                        debug!("already in error/shutdown state: ignoring error while purging outdated requests");
                    }
                    _ => return Some(self.into_error_state(err)),
                }
            }

            info!("transitioning to the next phase");
            self.next()
        }.instrument(span).await
    }
}
