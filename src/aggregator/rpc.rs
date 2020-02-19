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
use tokio::sync::{mpsc, oneshot};

#[tarpc::service]
trait AggregatorTarpcService {
    async fn select(id: ClientId, token: Token) -> Result<(), ()>;
    async fn reset() -> Result<(), ()>;
}

// NOTE: the server is cloned on every request, so cloning should
// remain cheap!
#[derive(Clone)]
struct AggregatorTarpcServer {
    select: mpsc::UnboundedSender<CoordinatorSelectRequest>,
    reset: mpsc::UnboundedSender<CoordinatorResetRequest>,
}

impl AggregatorTarpcServer {
    fn new() -> (Self, AggregatorTarpcServiceHandle) {
        let (select_tx, select_rx) = mpsc::unbounded_channel::<CoordinatorSelectRequest>();
        let (reset_tx, reset_rx) = mpsc::unbounded_channel::<CoordinatorResetRequest>();

        let server = AggregatorTarpcServer {
            select: select_tx,
            reset: reset_tx,
        };

        let handle = AggregatorTarpcServiceHandle::new(select_rx, reset_rx);

        (server, handle)
    }
}

type CoordinatorSelectRequest = ((ClientId, Token), oneshot::Sender<()>);
type CoordinatorResetRequest = oneshot::Sender<()>;

enum CoordinatorRequest {
    Select(CoordinatorSelectRequest),
    Reset(CoordinatorResetRequest),
}

struct AggregatorTarpcServiceHandle(Pin<Box<dyn Stream<Item = CoordinatorRequest>>>);

impl AggregatorTarpcServiceHandle {
    fn new(
        select: mpsc::UnboundedReceiver<CoordinatorSelectRequest>,
        reset: mpsc::UnboundedReceiver<CoordinatorResetRequest>,
    ) -> Self {
        Self(Box::pin(
            reset
                .map(CoordinatorRequest::Reset)
                .chain(select.map(CoordinatorRequest::Select)),
        ))
    }
}

impl Stream for AggregatorTarpcServiceHandle {
    type Item = CoordinatorRequest;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.0.as_mut().poll_next(cx)
    }
}

impl AggregatorTarpcService for AggregatorTarpcServer {
    type SelectFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;
    type ResetFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

    fn select(mut self, _: tarpc::context::Context, id: ClientId, token: Token) -> Self::SelectFut {
        let (tx, rx) = oneshot::channel();
        Box::pin(async move {
            self.select.send(((id, token), tx)).map_err(|_| ())?;
            rx.map_err(|_| ()).await
        })
    }

    fn reset(mut self, _: tarpc::context::Context) -> Self::ResetFut {
        let (tx, rx) = oneshot::channel();
        Box::pin(async move {
            self.reset.send(tx).map_err(|_| ())?;
            rx.map_err(|_| ()).await
        })
    }
}
