use std::{
    collections::HashMap,
    error::Error,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::{
    aggregator::rpc::{RpcHandle, RpcRequest},
    common::{ClientId, Token},
    coordinator,
};

use futures::{ready, stream::Stream};
use tokio::sync::mpsc;

/// A future that orchestrates the entire aggregator service.
struct Service<A>
where
    A: Aggregator,
{
    /// Clients that the coordinator selected for the current
    /// round. They can use their unique token to download the global
    /// weights and upload their own local results once they finished
    /// training.
    known_ids: HashMap<ClientId, Token>,

    /// The latest global weights as computed by the aggregator.
    global_weights: Arc<Vec<u8>>,

    /// The aggregator itself, which handles the weights or performs
    /// the aggregations.
    aggregator: A,

    /// A client for the coordinator RPC service.
    coordinator: coordinator::RpcClient,

    /// If the coordinator has open a connection to the aggregator's
    /// RPC server, the incoming requests are forwarded to this
    /// handle.
    rpc_requests: Option<RpcHandle>,

    /// The aggregator RPC server only accepts one client at a time,
    /// since we expect a single coordinator instance to connect. When
    /// a new connection is open, the `RpcHandle` for that new client
    /// is received from this channel.
    rpc_connections: mpsc::Receiver<RpcHandle>,
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
    fn poll_rpc_requests(&mut self, cx: &mut Context) {
        trace!("polling RPC requests");

        if self.rpc_requests.is_none() {
            trace!("no active RPC connection");
            return;
        }

        let mut stream = Pin::new(self.rpc_requests.as_mut().unwrap());
        loop {
            match stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(RpcRequest::Select(((id, token), resp_tx)))) => {
                    self.known_ids.insert(id, token);
                    if resp_tx.send(()).is_err() {
                        warn!("aggregator RPC service finished");
                        return;
                    }
                }
                Poll::Ready(Some(RpcRequest::Reset(resp_tx))) => {
                    self.known_ids = HashMap::new();
                    if resp_tx.send(()).is_err() {
                        warn!("aggregator RPC service finished");
                        return;
                    }
                }
                // The coordinator client disconnected. If the
                // coordinator reconnect to the RPC server, a new
                // RpcHandle will be forwarded to us.
                Poll::Ready(None) => {
                    debug!("RPC connection lost, now waiting for a new connection");
                    // The RpcHandle is of no use now. We'll have to wait for a
                    // new one, when a client reconnects.
                    self.rpc_requests = None;
                    return;
                }
                Poll::Pending => return,
            }
        }
    }

    fn poll_rpc_connections(&mut self, cx: &mut Context) -> Poll<()> {
        let mut stream = Pin::new(&mut self.rpc_connections);
        loop {
            match ready!(stream.as_mut().poll_next(cx)) {
                Some(handle) => {
                    debug!("new RPC connection");
                    self.rpc_requests = Some(handle);
                }
                None => {
                    return Poll::Ready(());
                }
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
        let pin = self.get_mut();
        ready!(pin.poll_rpc_connections(cx));
        pin.poll_rpc_requests(cx);
        Poll::Pending
    }
}
