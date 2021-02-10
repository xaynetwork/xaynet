use std::task::Poll;

use futures::task::Context;
use tower::Service;
use xaynet_core::message::Message;

use crate::{
    services::messages::{BoxedServiceFuture, ServiceError},
    state_machine::requests::RequestSender,
};

/// A service that hands the requests to the [`StateMachine`] that runs in the background.
///
/// [`StateMachine`]: crate::state_machine::StateMachine
#[derive(Debug, Clone)]
pub struct StateMachine {
    handle: RequestSender,
}

impl StateMachine {
    /// Create a new service with the given handle for forwarding
    /// requests to the state machine. The handle should be obtained
    /// via [`init()`].
    ///
    /// [`init()`]: crate::state_machine::initializer::StateMachineInitializer::init
    pub fn new(handle: RequestSender) -> Self {
        Self { handle }
    }
}

impl Service<Message> for StateMachine {
    type Response = ();
    type Error = ServiceError;
    type Future = BoxedServiceFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Message) -> Self::Future {
        let handle = self.handle.clone();
        Box::pin(async move {
            handle
                .request(req.into(), tracing::Span::none())
                .await
                .map_err(ServiceError::StateMachine)
        })
    }
}
