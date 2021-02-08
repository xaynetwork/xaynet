use async_trait::async_trait;
use tracing::{debug, Span};

use crate::{
    state_machine::{
        phases::{Phase, PhaseState, PhaseStateError},
        requests::{ResponseSender, StateMachineRequest},
        RequestError,
    },
    storage::Storage,
};

/// A trait that must be implemented by a state to handle a request.
#[async_trait]
pub trait Handler {
    /// Handles a request.
    ///
    /// # Errors
    /// Fails on PET and storage errors.
    async fn handle_request(&mut self, req: StateMachineRequest) -> Result<(), RequestError>;

    /// Checks whether enough requests have been processed successfully wrt the PET settings.
    fn has_enough_messages(&self) -> bool;

    /// Checks whether too many requests are processed wrt the PET settings.
    fn has_overmuch_messages(&self) -> bool;

    /// Increments the counter for accepted requests.
    fn increment_accepted(&mut self);

    /// Increments the counter for rejected requests.
    fn increment_rejected(&mut self);

    /// Increments the counter for discarded requests.
    fn increment_discarded(&mut self);
}

/// Implements all [`Handler`] methods for [`PhaseState`] except for [`handle_request()`].
///
/// Circumvents the infeasibility of default trait impls due to the dependency on internal state.
#[doc(hidden)]
#[macro_export]
macro_rules! impl_handler_for_phasestate {
    ($phase: ty) => {
        paste::paste! {
            fn has_enough_messages(&self) -> bool {
                self.private.accepted >= self.shared.state.[<$phase:lower>].count.min
            }

            fn has_overmuch_messages(&self) -> bool {
                self.private.accepted >= self.shared.state.[<$phase:lower>].count.max
            }

            fn increment_accepted(&mut self) {
                let phase = <Self as crate::state_machine::phases::Phase<_>>::NAME;
                self.private.accepted += 1;
                tracing::debug!(
                    "{} {} messages accepted (min {} and max {} required)",
                    self.private.accepted,
                    phase,
                    self.shared.state.[<$phase:lower>].count.min,
                    self.shared.state.[<$phase:lower>].count.max,
                );
                crate::accepted!(self.shared.state.round_id, phase);
            }

            fn increment_rejected(&mut self) {
                let phase = <Self as crate::state_machine::phases::Phase<_>>::NAME;
                self.private.rejected += 1;
                tracing::debug!("{} {} messages rejected", self.private.rejected, phase);
                crate::rejected!(self.shared.state.round_id, phase);
            }

            fn increment_discarded(&mut self) {
                let phase = <Self as crate::state_machine::phases::Phase<_>>::NAME;
                self.private.discarded += 1;
                tracing::debug!("{} {} messages discarded", self.private.discarded, phase);
                crate::discarded!(self.shared.state.round_id, phase);
            }
        }
    };
}

/// Implements `process()` for [`PhaseState`]`: `[`Handler`]
#[doc(hidden)]
#[macro_export]
macro_rules! impl_process_for_phasestate_handler {
    ($phase: ty) => {
        paste::paste! {
            impl<S> crate::state_machine::phases::PhaseState<$phase, S>
            where
                Self: crate::state_machine::phases::Handler
                    + crate::state_machine::phases::Phase<S>,
                S: crate::storage::Storage,
            {
                // Processes requests wrt the phase parameters.
                //
                // - Processes at most `count.max` requests during the time interval
                // `[now, now + time.min]`.
                // - Processes requests until there are enough (ie `count.min`) for the time
                // interval `[now + time.min, now + time.max]`.
                // - Aborts if either all connections were dropped or not enough requests were
                // processed until timeout.
                #[doc =
                    "Processes requests wrt the phase parameters.\n\n"
                    "- Processes at most `" [<$phase:lower>] ".count.max` requests during the time interval `[now, now + " [<$phase:lower>] ".time.min]`.\n"
                    "- Processes requests until there are enough (ie `" [<$phase:lower>] ".count.min`) for the time interval `[now + " [<$phase:lower>] ".time.min, now + " [<$phase:lower>] ".time.max]`.\n"
                    "- Aborts if either all connections were dropped or not enough requests were processed until timeout."
                ]
                async fn process(&mut self) -> Result<(), PhaseStateError> {
                    let crate::state_machine::coordinator::PhaseParameters { count, time } =
                        self.shared.state.[<$phase:lower>];
                    tracing::info!("processing requests");
                    tracing::debug!("processing for min {} and max {} seconds", time.min, time.max);
                    self.process_during(tokio::time::Duration::from_secs(time.min)).await?;

                    let time_left = time.max - time.min;
                    tokio::time::timeout(
                        tokio::time::Duration::from_secs(time_left),
                        self.process_until_enough()
                    ).await??;

                    tracing::info!(
                        "in total {} messages accepted (min {} and max {} required)",
                        self.private.accepted,
                        count.min,
                        count.max,
                    );
                    tracing::info!("in total {} messages rejected", self.private.rejected);
                    tracing::info!(
                        "in total {} messages discarded (purged not included)",
                        self.private.discarded,
                    );

                    Ok(())
                }
            }
        }
    };
}

impl<S, T> PhaseState<S, T>
where
    Self: Handler + Phase<T>,
    T: Storage,
{
    /// Processes requests for as long as the given duration.
    pub(in crate::state_machine::phases) async fn process_during(
        &mut self,
        dur: tokio::time::Duration,
    ) -> Result<(), PhaseStateError> {
        let deadline = tokio::time::sleep(dur);
        tokio::pin!(deadline);

        loop {
            tokio::select! {
                _ = &mut deadline => {
                    debug!("duration elapsed");
                    break Ok(());
                }
                next = self.next_request() => {
                    let (req, span, resp_tx) = next?;
                    self.process_single(req, span, resp_tx).await;
                }
            }
        }
    }

    /// Processes requests until there are enough.
    pub(in crate::state_machine::phases) async fn process_until_enough(
        &mut self,
    ) -> Result<(), PhaseStateError> {
        while !self.has_enough_messages() {
            let (req, span, resp_tx) = self.next_request().await?;
            self.process_single(req, span, resp_tx).await;
        }
        Ok(())
    }

    /// Processes a single request.
    ///
    /// The request is discarded if the maximum message count is reached, accepted if processed
    /// successfully and rejected otherwise.
    async fn process_single(
        &mut self,
        req: StateMachineRequest,
        span: Span,
        resp_tx: ResponseSender,
    ) {
        let _span_guard = span.enter();

        let response = if self.has_overmuch_messages() {
            self.increment_discarded();
            Err(RequestError::MessageDiscarded)
        } else {
            let response = self.handle_request(req).await;
            if response.is_ok() {
                self.increment_accepted();
            } else {
                self.increment_rejected();
            }
            response
        };

        // This may error out if the receiver has already been dropped but it doesn't matter for us.
        let _ = resp_tx.send(response);
    }
}
