use crate::{
    aggregator::rpc,
    common::client::{ClientId, Token},
    coordinator,
};
use bytes::Bytes;
use derive_more::From;
use futures::{ready, stream::Stream};
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tarpc::context::current as rpc_context;
use tokio::{
    stream::StreamExt,
    sync::{mpsc, oneshot},
};
use tracing_futures::Instrument;

/// A future that orchestrates the entire aggregator service.
// TODO: maybe add a HashSet or HashMap of clients who already
// uploaded their weights to prevent a client from uploading weights
// multiple times. Or we could just remove that ID from the
// `allowed_ids` map.

// TODO: maybe add a HashSet for clients that are already
// downloading/uploading, to prevent DoS attacks.
pub struct AggregatorService<A>
where
    A: Aggregator,
{
    /// Clients that the coordinator selected for the current
    /// round. They can use their unique token to download the global
    /// weights and upload their own local results once they finished
    /// training.
    allowed_ids: HashMap<ClientId, Token>,

    /// The latest global weights as computed by the aggregator.
    // NOTE: We could store this directly in the task that handles the
    // HTTP requests. I initially though that having it here would
    // make it easier to bypass the HTTP layer, which is convenient
    // for testing because we can simulate client with just
    // AggregatorHandles. But maybe that's just another layer of
    // complexity that is not worth it.
    global_weights: Bytes,

    /// The aggregator itself, which handles the weights or performs
    /// the aggregations.
    aggregator: A,

    /// A client for the coordinator RPC service.
    rpc_client: coordinator::rpc::Client,

    /// RPC requests from the coordinator.
    rpc_requests: rpc::RpcRequestsMux,

    aggregation_future: Option<AggregationFuture<A>>,

    api_rx: ApiRx,
}

/// This trait defines the methods that an aggregator should
/// implement.
pub trait Aggregator {
    // FIXME: we should obviously require the Error bound, but for now
    // it's convenient to be able to use () as error type
    type Error: Send + 'static + Sync;
    // type Error: Error;
    type AggregateFut: Future<Output = Result<Bytes, Self::Error>> + Unpin;
    type AddWeightsFut: Future<Output = Result<(), Self::Error>> + Unpin + Send + 'static;

    /// Check the validity of the given weights and if they are valid,
    /// add them to the set of weights to aggregate.
    fn add_weights(&mut self, weights: Bytes) -> Self::AddWeightsFut;

    /// Run the aggregator and return the result.
    fn aggregate(&mut self) -> Self::AggregateFut;
}

impl<A> AggregatorService<A>
where
    A: Aggregator,
{
    pub fn new(
        aggregator: A,
        rpc_client: coordinator::rpc::Client,
        rpc_requests: rpc::RpcRequestsMux,
    ) -> (Self, AggregatorServiceHandle) {
        let (handle, api_rx) = AggregatorServiceHandle::new();
        let service = Self {
            aggregator,
            rpc_requests,
            rpc_client,
            allowed_ids: HashMap::new(),
            global_weights: Bytes::new(),
            aggregation_future: None,
            api_rx,
        };
        (service, handle)
    }

    /// Handle the incoming requests.
    fn poll_api_rx(&mut self, cx: &mut Context) -> Poll<()> {
        trace!("polling API requests");
        loop {
            match ready!(Pin::new(&mut self.api_rx).poll_next(cx)) {
                Some(request) => self.handle_api_request(request),
                None => {
                    trace!("no more API request to handle");
                    return Poll::Ready(());
                }
            }
        }
    }

    fn handle_api_request(&mut self, request: Request) {
        match request {
            Request::Download(Credentials(id, token), response_tx) => {
                debug!("handling download request");
                if self
                    .allowed_ids
                    .get(&id)
                    .map(|expected_token| token == *expected_token)
                    .unwrap_or(false)
                {
                    let _ = response_tx.send(self.global_weights.clone());
                } else {
                    debug!("rejecting download request");
                }
            }
            Request::Upload(Credentials(id, token), bytes) => {
                debug!("handling upload request");
                let accept_upload = self
                    .allowed_ids
                    .get(&id)
                    .map(|expected_token| token == *expected_token)
                    .unwrap_or(false);

                if !accept_upload {
                    debug!("rejecting upload request");
                    return;
                }

                let mut rpc_client = self.rpc_client.clone();
                let fut = self.aggregator.add_weights(bytes);
                tokio::spawn(
                    async move {
                        let result = fut.await;
                        debug!("sending end training request to the coordinator");
                        rpc_client
                            .end_training(rpc_context(), id, result.is_ok())
                            .await
                            .map_err(|e| {
                                warn!(
                                    "failed to send end training request to the coordinator: {}",
                                    e
                                );
                            })
                    }
                    .instrument(trace_span!("end_training_rpc_request")),
                );
            }
        }
    }

    fn poll_rpc_requests(&mut self, cx: &mut Context) -> Poll<()> {
        trace!("polling RPC requests");

        loop {
            match ready!(Pin::new(&mut self.rpc_requests).poll_next(cx)) {
                Some(rpc_request) => self.handle_rpc_request(rpc_request),
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

    fn handle_rpc_request(&mut self, request: rpc::Request) {
        use rpc::Request::*;
        debug!("handling RPC request");
        match request {
            Select(select_request) => {
                let rpc::SelectRequest {
                    id,
                    token,
                    response_tx,
                } = select_request;
                info!("handling select request");
                self.allowed_ids.insert(id, token);
                if response_tx.send(()).is_err() {
                    warn!(client = %id, "RPC connection shut down, cannot send response back", );
                }
            }
            Aggregate(rpc::AggregateRequest { response_tx }) => {
                info!("handling aggregate request");
                self.allowed_ids = HashMap::new();

                self.aggregation_future = Some(AggregationFuture {
                    future: self.aggregator.aggregate(),
                    response_tx,
                });
            }
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn poll_aggregation(&mut self, cx: &mut Context) {
        // Check if we're waiting for an aggregation, ie whether
        // there's a future to poll.
        let future = if let Some(future) = self.aggregation_future.take() {
            future
        } else {
            trace!("no aggregation future running: skipping polling");
            return;
        };

        trace!("polling aggregation future");

        let AggregationFuture {
            mut future,
            response_tx,
        } = future;

        match Pin::new(&mut future).poll(cx) {
            Poll::Ready(Ok(weights)) => {
                info!("aggregation result is available, settings global weights");
                self.global_weights = weights;
                if response_tx.send(()).is_err() {
                    error!("failed to send aggregation response to RPC task: receiver dropped");
                }
            }
            Poll::Ready(Err(_)) => {
                // no need to send a response. By dropping the
                // `response_tx` channel, the RPC task will send
                // an error.
                error!("aggregation failed");
            }
            Poll::Pending => {
                debug!("aggregation future still running");
                self.aggregation_future = Some(AggregationFuture {
                    future,
                    response_tx,
                });
            }
        }
    }
}

struct AggregationFuture<A>
where
    A: Aggregator,
{
    future: A::AggregateFut,
    response_tx: oneshot::Sender<()>,
}

impl<A> Future for AggregatorService<A>
where
    A: Aggregator + Unpin,
{
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        trace!("polling AggregatorService");

        let pin = self.get_mut();

        if let Poll::Ready(_) = pin.poll_rpc_requests(cx) {
            return Poll::Ready(());
        }

        if let Poll::Ready(_) = pin.poll_api_rx(cx) {
            return Poll::Ready(());
        }

        pin.poll_aggregation(cx);

        Poll::Pending
    }
}

pub struct Credentials(ClientId, Token);

#[derive(Clone)]
pub struct AggregatorServiceHandle {
    upload_requests_tx: mpsc::UnboundedSender<(Credentials, Bytes)>,
    download_requests_tx: mpsc::UnboundedSender<(Credentials, oneshot::Sender<Bytes>)>,
}

impl AggregatorServiceHandle {
    fn new() -> (Self, ApiRx) {
        let (upload_requests_tx, upload_requests_rx) =
            mpsc::unbounded_channel::<(Credentials, Bytes)>();

        let (download_requests_tx, download_requests_rx) =
            mpsc::unbounded_channel::<(Credentials, oneshot::Sender<Bytes>)>();

        let handle = Self {
            upload_requests_tx,
            download_requests_tx,
        };
        let request_receiver = ApiRx::new(upload_requests_rx, download_requests_rx);
        (handle, request_receiver)
    }
}

pub struct ApiRx(Pin<Box<dyn Stream<Item = Request> + Send>>);

impl Stream for ApiRx {
    type Item = Request;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        trace!("polling ApiRx");
        self.0.as_mut().poll_next(cx)
    }
}

impl ApiRx {
    fn new(
        upload_requests_rx: mpsc::UnboundedReceiver<(Credentials, Bytes)>,
        download_requests_rx: mpsc::UnboundedReceiver<(Credentials, oneshot::Sender<Bytes>)>,
    ) -> Self {
        Self(Box::pin(
            download_requests_rx
                .map(Request::from)
                .merge(upload_requests_rx.map(Request::from)),
        ))
    }
}

#[derive(From)]
pub enum Request {
    Upload(Credentials, Bytes),
    Download(Credentials, oneshot::Sender<Bytes>),
}

impl AggregatorServiceHandle {
    pub async fn download(&self, id: ClientId, token: Token) -> Option<Bytes> {
        trace!("forwarding download request");

        let (tx, rx) = oneshot::channel();
        if self
            .download_requests_tx
            .send((Credentials(id, token), tx))
            .is_err()
        {
            warn!("failed to send download request: channel closed");
            return None;
        }
        trace!("forwarded download request, awaiting for response");

        rx.await.ok()
    }

    pub async fn upload(&self, id: ClientId, token: Token, weights: Bytes) {
        trace!("forwarding upload request");

        if self
            .upload_requests_tx
            .send((Credentials(id, token), weights))
            .is_err()
        {
            warn!("failed to send upload request: channel closed");
        }
    }
}
