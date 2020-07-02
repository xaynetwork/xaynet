mod error;
mod idle;
mod shutdown;
mod sum;
mod sum2;
mod unmask;
mod update;

pub use self::{
    error::StateError,
    idle::Idle,
    shutdown::Shutdown,
    sum::Sum,
    sum2::Sum2,
    unmask::Unmask,
    update::Update,
};

use crate::{
    state_machine::{
        coordinator::CoordinatorState,
        requests::{Request, RequestReceiver},
        StateMachine,
    },
    utils::trace::{Traceable, Traced},
    PetError,
};

use futures::StreamExt;
use tokio::sync::oneshot;

#[async_trait]
pub trait Phase<R> {
    async fn next(mut self) -> Option<StateMachine<R>>;
}

pub trait Handler<R> {
    fn handle_request(&mut self, req: R);
}

impl<R, S> Handler<Traced<Request>> for PhaseState<R, S>
where
    Self: Handler<Request>,
{
    /// Handle a sum, update or sum2 request.
    /// If the request is a update or sum2 request, the receiver of the response channel will
    /// receive a [`PetError::InvalidMessage`].
    fn handle_request(&mut self, req: Traced<Request>) {
        let span = req.span().clone();
        let _enter = span.enter();
        <Self as Handler<Request>>::handle_request(self, req.into_inner())
    }
}

pub struct PhaseState<R, S> {
    // Inner state
    inner: S,
    // Coordinator state
    coordinator_state: CoordinatorState,
    // Request receiver halve
    request_rx: RequestReceiver<R>,
}

// Functions that are available to all states
impl<R, S> PhaseState<R, S> {
    /// Receives the next [`Request`].
    /// Returns [`StateError::ChannelError`] when all sender halve have been dropped.
    async fn next_request(&mut self) -> Result<R, StateError> {
        debug!("waiting for the next incoming request");
        self.request_rx.next().await.ok_or_else(|| {
            error!("request receiver broken: senders have been dropped");
            StateError::ChannelError("all message senders have been dropped!")
        })
    }

    /// Handle an invalid request.
    fn handle_invalid_message(response_tx: oneshot::Sender<Result<(), PetError>>) {
        debug!("invalid message");
        // `send` returns an error if the receiver halve has already been dropped. This means that
        // the receiver is not interested in the response of the request. Therefore the error is
        // ignored.
        let _ = response_tx.send(Err(PetError::InvalidMessage));
    }
}
