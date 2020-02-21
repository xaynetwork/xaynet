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

use tokio::net::ToSocketAddrs;

use futures::{ready, stream::Stream};

/// A future that orchestrates the entire aggregator service.
pub struct AggregatorService<A, T>
where
    A: Aggregator,
    T: ToSocketAddrs + Send + Sync + 'static + Clone + Unpin,
{
    /// Clients that the coordinator selected for the current
    /// round. They can use their unique token to download the global
    /// weights and upload their own local results once they finished
    /// training.
    known_ids: HashMap<ClientId, Token>,

    /// The latest global weights as computed by the aggregator.
    global_weights: Arc<Vec<u8>>,

    // /// The aggregator itself, which handles the weights or performs
    // /// the aggregations.
    aggregator: A,

    ///// A client for the coordinator RPC service.
    coordinator_rpc: coordinator::rpc::Connection<T>,
    rpc_requests: rpc::RequestReceiver,
    // http_requests: aggregator::http::Handle,
}

// struct HttpServiceHandle {
//     start_training_requests: mpsc::Receiver<(Token, oneshot::Sender<Arc<Vec<u8>>>)>,
//     end_training_requests: mpsc::Receiver<(Token, Vec<u8>)>,
// }

#[async_trait]
/// This trait defines the methods that an aggregator should
/// implement.
pub trait Aggregator {
    type Error: Error;

    /// Check the validity of the given weights and if they are valid,
    /// add them to the set of weights to aggregate.
    async fn add_weights(&mut self, weights: Vec<u8>) -> Result<(), Self::Error>;

    /// Run the aggregator and return the result.
    async fn aggregate(&mut self) -> Result<Vec<u8>, Self::Error>;
}

impl<A, T> AggregatorService<A, T>
where
    A: Aggregator,
    T: ToSocketAddrs + Send + Sync + 'static + Clone + Unpin,
{
    pub fn new<U: ToSocketAddrs + Send + Sync + 'static>(
        aggregator: A,
        rpc_listen_addr: U,
        coordinator_rpc_addr: T,
    ) -> Self {
        let rpc_requests = rpc::run(rpc_listen_addr);
        Self {
            aggregator,
            rpc_requests,
            coordinator_rpc: coordinator::rpc::Connection::new(coordinator_rpc_addr),
            known_ids: HashMap::new(),
            global_weights: Arc::new(vec![]),
        }
    }

    fn poll_rpc_requests(&mut self, cx: &mut Context) -> Poll<()> {
        trace!("polling RPC requests");

        let mut stream = Pin::new(&mut self.rpc_requests);
        loop {
            match ready!(stream.as_mut().poll_next(cx)) {
                Some(rpc::Request::Select(((id, token), resp_tx))) => {
                    self.known_ids.insert(id, token);
                    if resp_tx.send(()).is_err() {
                        warn!("RPC connection shut down, cannot send response back");
                    }
                }
                Some(rpc::Request::Reset(resp_tx)) => {
                    self.known_ids = HashMap::new();
                    if resp_tx.send(()).is_err() {
                        warn!("RPC connection shut down, cannot send response back");
                    }
                }
                // The coordinator client disconnected. If the
                // coordinator reconnect to the RPC server, a new
                // AggregatorRpcHandle will be forwarded to us.
                None => {
                    warn!("RPC server shut down");
                    return Poll::Ready(());
                }
            }
        }
    }
}

impl<A, T> Future for AggregatorService<A, T>
where
    A: Aggregator + Unpin,
    T: ToSocketAddrs + Send + Sync + 'static + Clone + Unpin,
{
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let pin = self.get_mut();
        pin.poll_rpc_requests(cx)
    }
}
