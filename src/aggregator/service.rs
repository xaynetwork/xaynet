use std::{
    collections::HashMap,
    error::Error,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::{
    aggregator::rpc,
    common::{ClientId, Token},
    coordinator,
};

use futures::{ready, stream::Stream};

struct Service<A>
where
    A: Aggregator,
{
    known_ids: HashMap<ClientId, Token>,
    global_weights: Arc<Vec<u8>>,
    aggregator: A,
    coordinator: coordinator::RpcClient,
    rpc_requests: rpc::Handle,
    // http_requests: aggregator::http::Handle,
}

// struct HttpServiceHandle {
//     start_training_requests: mpsc::Receiver<(Token, oneshot::Sender<Arc<Vec<u8>>>)>,
//     end_training_requests: mpsc::Receiver<(Token, Vec<u8>)>,
// }

#[async_trait]
/// This trait defines the methods that an aggregator should
/// implement.
trait Aggregator {
    type Error: Error;

    /// Check the validity of the given weights and if they are valid,
    /// add them to the set of weights to aggregate.
    async fn add_weights(&mut self, weights: Vec<u8>) -> Result<(), Self::Error>;

    /// Run the aggregator and return the result.
    async fn aggregate(&mut self) -> Result<Vec<u8>, Self::Error>;
}

impl<A> Service<A>
where
    A: Aggregator,
{
    fn poll_rpc_requests(&mut self, cx: &mut Context) -> Poll<()> {
        let mut stream = Pin::new(&mut self.rpc_requests);
        loop {
            match ready!(stream.as_mut().poll_next(cx)) {
                Some(rpc::Request::Select(((id, token), resp_tx))) => {
                    self.known_ids.insert(id, token);
                    if resp_tx.send(()).is_err() {
                        warn!("aggregator RPC service finished");
                        return Poll::Ready(());
                    }
                }
                Some(rpc::Request::Reset(resp_tx)) => {
                    self.known_ids = HashMap::new();
                    if resp_tx.send(()).is_err() {
                        warn!("aggregator RPC service finished");
                        return Poll::Ready(());
                    }
                }
                None => return Poll::Ready(()),
            }
        }
    }
}

impl<A> Future for Service<A>
where
    A: Aggregator + Unpin,
{
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        unimplemented!()
    }
}
