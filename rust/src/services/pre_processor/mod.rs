mod sum;
pub use sum::SumPreProcessorService;

mod update;
pub use update::UpdatePreProcessorService;

mod sum2;
pub use sum2::Sum2PreProcessorService;

use std::{pin::Pin, task::Poll};

use futures::{
    future::{self, Future},
    task::Context,
};
use tower::Service;

use crate::{
    coordinator::{Phase, RoundParameters},
    events::{Event, EventListener, EventSubscriber},
    message::{HeaderOwned, MessageOwned, PayloadOwned, Sum2Owned, SumOwned, UpdateOwned},
    services::{
        error::{RequestFailed, ServiceError},
        state_machine::StateMachineRequest,
    },
};

/// Route the request to the service that is ready to process it,
/// depending on the current coordinator phase.
pub struct PreProcessorService {
    params_listener: EventListener<RoundParameters>,
    /// A stream that receives phase updates
    phase_listener: EventListener<Phase>,
    /// Latest phase event the service has received
    latest_phase_event: Event<Phase>,
    /// Inner service to handle sum messages
    sum: SumPreProcessorService,
    /// Inner service to handle update messages
    update: UpdatePreProcessorService,
    /// Inner service to handle sum2 messages
    sum2: Sum2PreProcessorService,
}

impl PreProcessorService {
    pub fn new(subscriber: &EventSubscriber) -> Self {
        Self {
            params_listener: subscriber.params_listener(),
            phase_listener: subscriber.phase_listener(),
            latest_phase_event: subscriber.phase_listener().get_latest(),
            sum: SumPreProcessorService,
            update: UpdatePreProcessorService,
            sum2: Sum2PreProcessorService,
        }
    }
}

type SumRequest = (HeaderOwned, SumOwned, RoundParameters);
type UpdateRequest = (HeaderOwned, UpdateOwned, RoundParameters);
type Sum2Request = (HeaderOwned, Sum2Owned, RoundParameters);

pub type PreProcessorRequest = MessageOwned;
pub type PreProcessorResponse = Result<StateMachineRequest, RequestFailed>;

impl Service<PreProcessorRequest> for PreProcessorService {
    type Response = PreProcessorResponse;
    type Error = ServiceError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, ServiceError>> + 'static + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.latest_phase_event = self.phase_listener.get_latest();
        match self.latest_phase_event.event {
            Phase::Sum => self.sum.poll_ready(cx).map_err(Into::into),
            Phase::Update => self.update.poll_ready(cx).map_err(Into::into),
            Phase::Sum2 => self.sum2.poll_ready(cx).map_err(Into::into),
        }
    }

    fn call(&mut self, message: PreProcessorRequest) -> Self::Future {
        let MessageOwned { header, payload } = message;
        // `call()` is only called after `poll_ready()` returned
        // `Poll::Ready` so at this point, phase is `Some`
        match (self.latest_phase_event.event, payload) {
            (Phase::Sum, PayloadOwned::Sum(sum)) => {
                let req = (header, sum, self.params_listener.get_latest().event);
                let fut = self.sum.call(req);
                Box::pin(async move { fut.await.map_err(Into::into).map(Into::into) })
            }
            (Phase::Update, PayloadOwned::Update(update)) => {
                let req = (header, update, self.params_listener.get_latest().event);
                let fut = self.update.call(req);
                Box::pin(async move { fut.await.map_err(Into::into).map(Into::into) })
            }
            (Phase::Sum2, PayloadOwned::Sum2(sum2)) => {
                let req = (header, sum2, self.params_listener.get_latest().event);
                let fut = self.sum2.call(req);
                Box::pin(async move { fut.await.map_err(Into::into).map(Into::into) })
            }
            _ => Box::pin(future::ready(Ok(Err(RequestFailed::UnexpectedMessage)))),
        }
    }
}
