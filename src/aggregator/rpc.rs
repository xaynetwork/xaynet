use crate::common::{ClientId, Token};
use futures::future::TryFutureExt;
use std::{future::Future, pin::Pin};
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
    ids: mpsc::Sender<((ClientId, Token), oneshot::Sender<()>)>,
    reset: mpsc::Sender<((), oneshot::Sender<()>)>,
}

impl AggregatorTarpcService for AggregatorTarpcServer {
    type SelectFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;
    type ResetFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

    fn select(mut self, _: tarpc::context::Context, id: ClientId, token: Token) -> Self::SelectFut {
        let (tx, rx) = oneshot::channel();
        Box::pin(async move {
            self.ids
                .send(((id, token), tx))
                .map_err(|_| ())
                .and_then(|_| rx.map_err(|_| ()))
                .await
        })
    }

    fn reset(mut self, _: tarpc::context::Context) -> Self::ResetFut {
        let (tx, rx) = oneshot::channel();
        Box::pin(async move {
            self.reset
                .send(((), tx))
                .map_err(|_| ())
                .and_then(|_| rx.map_err(|_| ()))
                .await
        })
    }
}
