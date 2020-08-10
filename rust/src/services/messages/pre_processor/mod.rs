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
use thiserror::Error;
use tower::Service;

use crate::{
    message::{message::MessageOwned, payload::PayloadOwned},
    state_machine::{
        coordinator::RoundParameters,
        events::{Event, EventListener, EventSubscriber},
        phases::PhaseName,
    },
    utils::request::Request,
};

/// A service for performing sanity checks and preparing incoming
/// requests to be handled by the state machine.
pub struct PreProcessorService {
    params_listener: EventListener<RoundParameters>,
    /// A stream that receives phase updates
    phase_listener: EventListener<PhaseName>,
    /// Latest phase event the service has received
    latest_phase_event: Event<PhaseName>,
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

/// Request type for [`PreProcessorService`]
pub type PreProcessorRequest = Request<MessageOwned>;

/// Response type for [`PreProcessorService`]
pub type PreProcessorResponse = Result<MessageOwned, PreProcessorError>;

impl Service<PreProcessorRequest> for PreProcessorService {
    type Response = PreProcessorResponse;
    type Error = std::convert::Infallible;

    #[allow(clippy::type_complexity)]
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + 'static + Send + Sync>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.latest_phase_event = self.phase_listener.get_latest();
        match self.latest_phase_event.event {
            PhaseName::Sum => self.sum.poll_ready(cx),
            PhaseName::Update => self.update.poll_ready(cx),
            PhaseName::Sum2 => self.sum2.poll_ready(cx),
            _ => Poll::Ready(Ok(())),
        }
    }

    fn call(&mut self, req: PreProcessorRequest) -> Self::Future {
        let MessageOwned { header, payload } = req.into_inner();
        match (self.latest_phase_event.event, payload) {
            (PhaseName::Sum, PayloadOwned::Sum(sum)) => {
                let req = (header, sum, self.params_listener.get_latest().event);
                let fut = self.sum.call(req);
                Box::pin(fut)
            }
            (PhaseName::Update, PayloadOwned::Update(update)) => {
                let req = (header, update, self.params_listener.get_latest().event);
                let fut = self.update.call(req);
                Box::pin(fut)
            }
            (PhaseName::Sum2, PayloadOwned::Sum2(sum2)) => {
                let req = (header, sum2, self.params_listener.get_latest().event);
                let fut = self.sum2.call(req);
                Box::pin(fut)
            }
            _ => Box::pin(future::ready(Ok(Err(PreProcessorError::UnexpectedMessage)))),
        }
    }
}

/// Error type for [`PreProcessorService`]
#[derive(Error, Debug)]
pub enum PreProcessorError {
    #[error("Invalid sum signature")]
    InvalidSumSignature,

    #[error("Invalid update signature")]
    InvalidUpdateSignature,

    #[error("Not eligible for sum task")]
    NotSumEligible,

    #[error("Not eligible for update task")]
    NotUpdateEligible,

    #[error("The message was rejected because the coordinator did not expect it")]
    UnexpectedMessage,

    #[error("Internal error")]
    InternalError,
}
