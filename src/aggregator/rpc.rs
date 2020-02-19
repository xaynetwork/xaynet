use crate::common::{ClientId, Token};
use futures::{
    future::TryFutureExt,
    stream::{Stream, StreamExt},
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot::{channel, Sender},
};

#[tarpc::service]
/// Definition of the methods exposed by the aggregator RPC service.
pub trait Service {
    /// Notify the aggregator that the given client has been selected
    /// and should use the given token to download the global weights
    /// and upload their local weights.
    async fn select(id: ClientId, token: Token) -> Result<(), ()>;

    /// Notify the aggregator that it should clear its pool of client
    /// IDs and tokens. This should be called before starting a new
    /// round.
    async fn reset() -> Result<(), ()>;
}

// NOTE: the server is cloned on every request, so cloning should
// remain cheap!
#[derive(Clone)]
struct Server {
    select: UnboundedSender<SelectRequest>,
    reset: UnboundedSender<ResetRequest>,
}

impl Server {
    fn new() -> (Self, Handle) {
        let (select_tx, select_rx) = unbounded_channel::<SelectRequest>();
        let (reset_tx, reset_rx) = unbounded_channel::<ResetRequest>();

        let server = Server {
            select: select_tx,
            reset: reset_tx,
        };

        let handle = Handle::new(select_rx, reset_rx);

        (server, handle)
    }
}

/// An incoming [`Service::select`] RPC request
pub type SelectRequest = ((ClientId, Token), Sender<()>);
/// An incoming [`Service::reset`] RPC request
pub type ResetRequest = Sender<()>;

/// An incoming RPC request
pub enum Request {
    /// An incoming [`Service::select`] RPC request
    Select(SelectRequest),
    /// An incoming [`Service::reset`] RPC request
    Reset(ResetRequest),
}

/// A handle to receive the RPC requests received by the RPC
/// [`Service`].
pub struct Handle(Pin<Box<dyn Stream<Item = Request>>>);

impl Handle {
    fn new(
        select: UnboundedReceiver<SelectRequest>,
        reset: UnboundedReceiver<ResetRequest>,
    ) -> Self {
        Self(Box::pin(
            reset.map(Request::Reset).chain(select.map(Request::Select)),
        ))
    }
}

impl Stream for Handle {
    type Item = Request;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.0.as_mut().poll_next(cx)
    }
}

impl Service for Server {
    type SelectFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;
    type ResetFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

    fn select(self, _: tarpc::context::Context, id: ClientId, token: Token) -> Self::SelectFut {
        let (tx, rx) = channel();
        Box::pin(async move {
            self.select.send(((id, token), tx)).map_err(|_| ())?;
            rx.map_err(|_| ()).await
        })
    }

    fn reset(self, _: tarpc::context::Context) -> Self::ResetFut {
        let (tx, rx) = channel();
        Box::pin(async move {
            self.reset.send(tx).map_err(|_| ())?;
            rx.map_err(|_| ()).await
        })
    }
}
