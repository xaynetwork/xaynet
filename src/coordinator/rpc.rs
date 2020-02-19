use crate::common::ClientId;
use futures::future::TryFutureExt;
use std::{future::Future, pin::Pin};
use tokio::sync::{mpsc, oneshot};

#[tarpc::service]
pub trait CoordinatorTarpc {
    async fn end_training(id: ClientId) -> Result<(), ()>;
}

// NOTE: the server is cloned on every request, so cloning should
// remain cheap!
#[derive(Clone)]
pub struct CoordinatorTarpcServer {
    ids: mpsc::Sender<(ClientId, oneshot::Sender<()>)>,
}

impl CoordinatorTarpc for CoordinatorTarpcServer {
    type EndTrainingFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

    fn end_training(mut self, _: tarpc::context::Context, id: ClientId) -> Self::EndTrainingFut {
        let (tx, rx) = oneshot::channel();
        // FIXME: the async block is here to force `self.ids` to be taken
        // by value instead of mutably borrowed, so that the future is
        // 'static. But I don't understand why the compiler forces us
        // to do that...
        Box::pin(async move {
            self.ids
                .send((id, tx))
                .map_err(|_| ())
                .and_then(|_| rx.map_err(|_| ()))
                .await
        })
    }
}
