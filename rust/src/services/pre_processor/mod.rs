mod sum;
pub use sum::SumPreProcessorService;

mod update;
pub use update::UpdatePreProcessorService;

use std::{pin::Pin, task::Poll};

use anyhow::anyhow;
use futures::{
    future::{self, Future},
    stream::Stream,
    task::Context,
};
use tower::Service;

use crate::{
    coordinator::{CoordinatorWatcher, Phase, RoundParameters, WatcherStream},
    message::{HeaderOwned, MessageOwned, PayloadOwned, SumOwned, UpdateOwned},
    services::{
        error::{RequestFailed, ServiceError},
        state_machine::StateMachineRequest,
    },
};

/// Route the request to the service that is ready to process it,
/// depending on the current coordinator phase.
pub struct PreProcessorService {
    watcher: CoordinatorWatcher,
    /// A stream that receives phase updates
    phases: WatcherStream<Phase>,
    /// Latest phase the service is aware of
    current_phase: Option<Phase>,
    /// Inner service to handle sum messages
    sum: SumPreProcessorService,
    /// Inner service to handle update messages
    update: UpdatePreProcessorService,
}

impl PreProcessorService {
    pub fn new(watcher: CoordinatorWatcher) -> Self {
        let phases = watcher.phase_stream();
        Self {
            watcher,
            phases,
            current_phase: None,
            sum: SumPreProcessorService,
            update: UpdatePreProcessorService,
        }
    }
}

type SumRequest = (HeaderOwned, SumOwned, RoundParameters);
type UpdateRequest = (HeaderOwned, UpdateOwned, RoundParameters);
// type Sum2Request = (HeaderOwned, Sum2Owned, RoundParameters);

pub type PreProcessorRequest = MessageOwned;
pub type PreProcessorResponse = Result<StateMachineRequest, RequestFailed>;

impl Service<PreProcessorRequest> for PreProcessorService {
    type Response = PreProcessorResponse;
    type Error = ServiceError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, ServiceError>> + 'static + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match Pin::new(&mut self.phases).poll_next(cx) {
            Poll::Ready(Some(phase)) => {
                self.current_phase = Some(phase);
            }
            Poll::Ready(None) => {
                return Poll::Ready(Err(anyhow!("cannot receive phase from coordinator")));
            }
            Poll::Pending => {}
        }
        match self.current_phase {
            None => Poll::Pending,
            Some(Phase::Sum) => self.sum.poll_ready(cx).map_err(Into::into),
            Some(Phase::Update) => self.update.poll_ready(cx).map_err(Into::into),
            // TODO: sum2 phase
            _ => unimplemented!(),
        }
    }

    fn call(&mut self, message: PreProcessorRequest) -> Self::Future {
        let MessageOwned { header, payload } = message;
        // `call()` is only called after `poll_ready()` returned
        // `Poll::Ready` so at this point, phase is `Some`
        match (self.current_phase.unwrap(), payload) {
            (Phase::Sum, PayloadOwned::Sum(sum)) => {
                let req = (header, sum, self.watcher.get_round_params());
                let fut = self.sum.call(req);
                Box::pin(async move { fut.await.map_err(Into::into).map(Into::into) })
            }
            (Phase::Update, PayloadOwned::Update(update)) => {
                let req = (header, update, self.watcher.get_round_params());
                let fut = self.update.call(req);
                Box::pin(async move { fut.await.map_err(Into::into).map(Into::into) })
            }
            // TODO: other cases
            _ => Box::pin(future::ready(Ok(Err(RequestFailed::UnexpectedMessage)))),
        }
    }
}
