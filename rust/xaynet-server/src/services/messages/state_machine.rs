use std::{pin::Pin, task::Poll};

use futures::{future::Future, task::Context};
use tower::Service;
use xaynet_core::message::Message;

use crate::{
    state_machine::{requests::RequestSender, StateMachineResult},
    utils::Request,
};

pub use crate::state_machine::{StateMachineError, StateMachineResult as StateMachineResponse};

/// A service that hands the requests to the state machine
/// ([`crate::state_machine::StateMachine`]) that runs in the
/// background.
pub struct StateMachineService {
    handle: RequestSender,
}

impl StateMachineService {
    /// Create a new service with the given handle for forwarding
    /// requests to the state machine. The handle should be obtained
    /// via [`crate::state_machine::StateMachine::new`]
    pub fn new(handle: RequestSender) -> Self {
        Self { handle }
    }
}

/// Request type for [`StateMachineService`]
pub type StateMachineRequest = Request<Message>;

impl Service<StateMachineRequest> for StateMachineService {
    type Response = StateMachineResult;
    type Error = ::std::convert::Infallible;
    #[allow(clippy::type_complexity)]
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + 'static + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: StateMachineRequest) -> Self::Future {
        let handle = self.handle.clone();
        Box::pin(async move { Ok(handle.request(req).await) })
    }
}
