use crate::common::{ClientId, Token};
use futures::future::TryFutureExt;
use std::{future::Future, pin::Pin};
use tokio::sync::{mpsc, oneshot};

#[tarpc::service]
trait AggregatorTarpcService {
    async fn select(id: ClientId, token: Token) -> Result<(), ()>;
    async fn reset() -> Result<(), ()>;
}

#[derive(Clone)]
struct AggregatorTarpcServer {
    ids: mpsc::Sender<((ClientId, Token), oneshot::Sender<()>)>,
    reset: mpsc::Sender<((), oneshot::Sender<()>)>,
}

impl AggregatorTarpcService for AggregatorTarpcServer {
    type SelectFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;
    type ResetFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

    fn select(self, _: tarpc::context::Context, id: ClientId, token: Token) -> Self::SelectFut {
        let (tx, rx) = oneshot::channel();
        let mut ids = self.ids.clone();
        // FIXME: the async block is here to force `ids` to be taken
        // by value instead of mutably borrowed, so that the future is
        // 'static. But I don't understand why the compiler forces us
        // to do that...
        Box::pin(async move {
            ids.send(((id, token), tx))
                .map_err(|_| ())
                .and_then(|_| rx.map_err(|_| ()))
                .await
        })
    }

    fn reset(self, _: tarpc::context::Context) -> Self::ResetFut {
        let (tx, rx) = oneshot::channel();
        let mut reset = self.reset.clone();
        // FIXME: the async block is here to force `ids` to be taken
        // by value instead of mutably borrowed, so that the future is
        // 'static. But I don't understand why the compiler forces us
        // to do that...
        Box::pin(async move {
            reset
                .send(((), tx))
                .map_err(|_| ())
                .and_then(|_| rx.map_err(|_| ()))
                .await
        })
    }
}
