use async_trait::async_trait;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, Span};

use crate::{
    accepted,
    discarded,
    rejected,
    state_machine::{
        coordinator::{CountParameters, PhaseParameters},
        phases::{Phase, PhaseError, PhaseState},
        requests::{RequestError, ResponseSender, StateMachineRequest},
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
}

/// A counter to keep track of handled messages.
struct Counter {
    /// The minimal number of successfully processed messages.
    min: u64,
    /// The maximal number of successfully processed messages.
    max: u64,
    /// The number of messages successfully processed.
    accepted: u64,
    /// The number of messages failed to processed.
    rejected: u64,
    /// The number of messages discarded without being processed.
    discarded: u64,
}

impl AsMut<Counter> for Counter {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl Counter {
    /// Creates a new message counter.
    fn new(CountParameters { min, max }: CountParameters) -> Self {
        Self {
            min,
            max,
            accepted: 0,
            rejected: 0,
            discarded: 0,
        }
    }

    /// Checks whether enough requests have been processed successfully wrt the PET settings.
    fn has_enough_messages(&self) -> bool {
        self.accepted >= self.min
    }

    /// Checks whether too many requests are processed wrt the PET settings.
    fn has_overmuch_messages(&self) -> bool {
        self.accepted >= self.max
    }

    /// Increments the counter for accepted requests.
    fn increment_accepted(&mut self) {
        self.accepted += 1;
        debug!(
            "{} messages accepted (min {} and max {} required)",
            self.accepted, self.min, self.max,
        );
    }

    /// Increments the counter for rejected requests.
    fn increment_rejected(&mut self) {
        self.rejected += 1;
        debug!("{} messages rejected", self.rejected);
    }

    /// Increments the counter for discarded requests.
    fn increment_discarded(&mut self) {
        self.discarded += 1;
        debug!("{} messages discarded", self.discarded);
    }
}

impl<S, T> PhaseState<S, T>
where
    T: Storage,
    Self: Phase<T> + Handler,
{
    /// Processes requests wrt the phase parameters.
    ///
    /// - Processes at most `count.max` requests during the time interval `[now, now + time.min]`.
    /// - Processes requests until there are enough (ie `count.min`) for the time interval
    /// `[now + time.min, now + time.max]`.
    /// - Aborts if either all connections were dropped or not enough requests were processed until
    /// timeout.
    pub(super) async fn process(
        &mut self,
        PhaseParameters { count, time }: PhaseParameters,
    ) -> Result<(), PhaseError> {
        let mut counter = Counter::new(count);

        info!("processing requests");
        debug!(
            "processing for min {} and max {} seconds",
            time.min, time.max
        );
        self.process_during(Duration::from_secs(time.min), counter.as_mut())
            .await?;

        let time_left = time.max - time.min;
        timeout(
            Duration::from_secs(time_left),
            self.process_until_enough(counter.as_mut()),
        )
        .await??;

        info!(
            "in total {} messages accepted (min {} and max {} required)",
            counter.accepted, counter.min, counter.max,
        );
        info!("in total {} messages rejected", counter.rejected);
        info!(
            "in total {} messages discarded (purged not included)",
            counter.discarded,
        );

        Ok(())
    }

    /// Processes requests for as long as the given duration.
    async fn process_during(
        &mut self,
        dur: tokio::time::Duration,
        counter: &mut Counter,
    ) -> Result<(), PhaseError> {
        let deadline = tokio::time::sleep(dur);
        tokio::pin!(deadline);

        loop {
            tokio::select! {
                biased;

                _ = &mut deadline => {
                    debug!("duration elapsed");
                    break Ok(());
                }
                next = self.next_request() => {
                    let (req, span, resp_tx) = next?;
                    self.process_single(req, span, resp_tx, counter).await;
                }
            }
        }
    }

    /// Processes requests until there are enough.
    async fn process_until_enough(&mut self, counter: &mut Counter) -> Result<(), PhaseError> {
        while !counter.has_enough_messages() {
            let (req, span, resp_tx) = self.next_request().await?;
            self.process_single(req, span, resp_tx, counter).await;
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
        counter: &mut Counter,
    ) {
        let _span_guard = span.enter();

        let response = if counter.has_overmuch_messages() {
            counter.increment_discarded();
            discarded!(self.shared.state.round_id, Self::NAME);
            Err(RequestError::MessageDiscarded)
        } else {
            let response = self.handle_request(req).await;
            if response.is_ok() {
                counter.increment_accepted();
                accepted!(self.shared.state.round_id, Self::NAME);
            } else {
                counter.increment_rejected();
                rejected!(self.shared.state.round_id, Self::NAME);
            }
            response
        };

        // This may error out if the receiver has already been dropped but it doesn't matter for us.
        let _ = resp_tx.send(response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        // 0 accepted
        let mut counter = Counter::new(CountParameters { min: 1, max: 3 });
        assert!(!counter.has_enough_messages());
        assert!(!counter.has_overmuch_messages());

        // 1 accepted
        counter.increment_accepted();
        assert!(counter.has_enough_messages());
        assert!(!counter.has_overmuch_messages());

        // 2 accepted
        counter.increment_accepted();
        assert!(counter.has_enough_messages());
        assert!(!counter.has_overmuch_messages());

        // 3 accepted
        counter.increment_accepted();
        assert!(counter.has_enough_messages());
        assert!(counter.has_overmuch_messages());
    }
}
