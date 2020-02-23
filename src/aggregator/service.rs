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
pub struct AggregatorService<A>
where
    A: Aggregator,
{
    /// Clients that the coordinator selected for the current
    /// round. They can use their unique token to download the global
    /// weights and upload their own local results once they finished
    /// training.
    allowed_ids: HashMap<ClientId, Token>,

    // TODO: maybe add a HashSet or HashMap of clients who already
    // uploaded their weights to prevent a client from uploading
    // weights multiple times. Or we could just remove that ID from
    // the `allowed_ids` map.

    // TODO: maybe add a HashSet for clients that are already
    // downloading/uploading, to prevent DoS attacks.
    /// The latest global weights as computed by the aggregator.
    global_weights: Arc<Vec<u8>>,

    // /// The aggregator itself, which handles the weights or performs
    // /// the aggregations.
    aggregator: A,

    ///// A client for the coordinator RPC service.
    coordinator_rpc: Option<coordinator::rpc::Client>,
    coordinator_rpc_connection: Option<coordinator::rpc::ConnectFuture>,
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

impl<A> AggregatorService<A>
where
    A: Aggregator,
{
    pub fn new<
        T: Clone + ToSocketAddrs + Send + Sync + 'static + Unpin,
        U: ToSocketAddrs + Send + Sync + 'static,
    >(
        aggregator: A,
        rpc_listen_addr: U,
        coordinator_rpc_addr: T,
    ) -> Self {
        let rpc_requests = rpc::run(rpc_listen_addr);
        Self {
            aggregator,
            rpc_requests,
            coordinator_rpc: None,
            coordinator_rpc_connection: Some(coordinator::rpc::ConnectFuture::new(
                coordinator_rpc_addr,
            )),
            allowed_ids: HashMap::new(),
            global_weights: Arc::new(vec![]),
        }
    }

    fn poll_rpc_requests(&mut self, cx: &mut Context) -> Poll<()> {
        trace!("polling RPC requests");

        let mut stream = Pin::new(&mut self.rpc_requests);
        loop {
            match ready!(stream.as_mut().poll_next(cx)) {
                Some(rpc::Request::Select(((id, token), resp_tx))) => {
                    info!("handling rpc request: select {}", id);
                    self.allowed_ids.insert(id, token);
                    if resp_tx.send(()).is_err() {
                        warn!("RPC connection shut down, cannot send response back");
                    }
                }
                Some(rpc::Request::Aggregate(resp_tx)) => {
                    info!("handling rpc request: aggregate");
                    self.allowed_ids = HashMap::new();
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

impl<A> Future for AggregatorService<A>
where
    A: Aggregator + Unpin,
{
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        trace!("polling AggregatorService");
        let pin = self.get_mut();

        // This only runs when the aggregator starts
        if let Some(ref mut connection) = pin.coordinator_rpc_connection {
            match Pin::new(connection).poll(cx) {
                Poll::Ready(Ok(client)) => {
                    pin.coordinator_rpc = Some(client);
                    pin.coordinator_rpc_connection = None;
                }
                Poll::Ready(Err(e)) => {
                    error!("failed to connect RPC client: {}", e);
                    return Poll::Ready(());
                }
                _ => {}
            }
        }

        if let Poll::Ready(_) = pin.poll_rpc_requests(cx) {
            return Poll::Ready(());
        }

        Poll::Pending
    }
}
