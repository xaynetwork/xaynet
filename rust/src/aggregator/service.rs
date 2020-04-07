use crate::{
    common::client::{ClientId, Credentials, Token},
    coordinator,
};
use bytes::Bytes;
use derive_more::From;
use futures::{ready, stream::Stream};
use std::{
    collections::HashMap,
    error::Error,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tarpc::context::current as rpc_context;
use thiserror::Error;
use tokio::{
    stream::StreamExt,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};
use tracing_futures::Instrument;

/// A future that orchestrates the entire aggregator service.
// TODO: maybe add a HashSet or HashMap of clients who already
// uploaded their weights to prevent a client from uploading weights
// multiple times. Or we could just remove that ID from the
// `allowed_ids` map.

// TODO: maybe add a HashSet for clients that are already
// downloading/uploading, to prevent DoS attacks.
pub struct Service<A>
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

    requests: ServiceRequests<A>,

    aggregation_future: Option<AggregationFuture<A>>,
}

/// This trait defines the methods that an aggregator should
/// implement.
pub trait Aggregator {
    type Error: Error + Send + 'static + Sync;
    type AggregateFut: Future<Output = Result<Bytes, Self::Error>> + Unpin;
    type AddWeightsFut: Future<Output = Result<(), Self::Error>> + Unpin + Send + 'static;

    /// Check the validity of the given weights and if they are valid,
    /// add them to the set of weights to aggregate.
    fn add_weights(&mut self, weights: Bytes) -> Self::AddWeightsFut;

    /// Run the aggregator and return the result.
    fn aggregate(&mut self) -> Self::AggregateFut;
}

impl<A> Service<A>
where
    A: Aggregator,
{
    pub fn new(
        aggregator: A,
        rpc_client: coordinator::rpc::Client,
        requests: ServiceRequests<A>,
    ) -> Self {
        Self {
            aggregator,
            requests,
            rpc_client,
            allowed_ids: HashMap::new(),
            global_weights: Bytes::new(),
            aggregation_future: None,
        }
    }

    /// Handle the incoming requests.
    fn poll_requests(&mut self, cx: &mut Context) -> Poll<()> {
        trace!("polling requests");
        loop {
            match ready!(Pin::new(&mut self.requests).poll_next(cx)) {
                Some(request) => self.handle_request(request),
                None => {
                    trace!("no more request to handle");
                    return Poll::Ready(());
                }
            }
        }
    }

    fn handle_download_request(&mut self, request: DownloadRequest) {
        debug!("handling download request");
        let DownloadRequest {
            credentials,
            response_tx,
        } = request;
        if self
            .allowed_ids
            .get(credentials.id())
            .map(|expected_token| credentials.token() == expected_token)
            .unwrap_or(false)
        {
            let _ = response_tx.send(Ok(self.global_weights.clone()));
        } else {
            warn!("rejecting download request");
            let _ = response_tx.send(Err(DownloadError::Unauthorized));
        }
    }

    fn handle_upload_request(&mut self, request: UploadRequest) {
        debug!("handling upload request");
        let UploadRequest { credentials, data } = request;
        let accept_upload = self
            .allowed_ids
            .get(credentials.id())
            .map(|expected_token| credentials.token() == expected_token)
            .unwrap_or(false);

        if !accept_upload {
            warn!("rejecting upload request");
            return;
        }

        let mut rpc_client = self.rpc_client.clone();
        let fut = self.aggregator.add_weights(data);
        tokio::spawn(
            async move {
                let result = fut.await;
                debug!("sending end training request to the coordinator");
                rpc_client
                    .end_training(rpc_context(), *credentials.id(), result.is_ok())
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

    fn handle_request(&mut self, request: Request<A>) {
        match request {
            Request::Download(req) => self.handle_download_request(req),
            Request::Upload(req) => self.handle_upload_request(req),
            Request::Select(req) => self.handle_select_request(req),
            Request::Aggregate(req) => self.handle_aggregate_request(req),
        }
    }

    fn handle_aggregate_request(&mut self, request: AggregateRequest<A>) {
        info!("handling aggregate request");
        let AggregateRequest { response_tx } = request;
        self.allowed_ids = HashMap::new();

        self.aggregation_future = Some(AggregationFuture {
            future: self.aggregator.aggregate(),
            response_tx,
        });
    }
    fn handle_select_request(&mut self, request: SelectRequest<A>) {
        info!("handling select request");
        let SelectRequest {
            credentials,
            response_tx,
        } = request;
        let (id, token) = credentials.into_parts();
        self.allowed_ids.insert(id, token);
        if response_tx.send(Ok(())).is_err() {
            warn!("failed to send reponse: channel closed");
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

        let result = match Pin::new(&mut future).poll(cx) {
            Poll::Ready(Ok(weights)) => {
                info!("aggregation succeeded, settings global weights");
                self.global_weights = weights;
                Ok(())
            }
            Poll::Ready(Err(e)) => {
                error!(error = %e, "aggregation failed");
                Err(e)
            }
            Poll::Pending => {
                debug!("aggregation future still running");
                self.aggregation_future = Some(AggregationFuture {
                    future,
                    response_tx,
                });
                return;
            }
        };
        if response_tx.send(result).is_err() {
            error!("failed to send aggregation response to RPC task: receiver dropped");
        }
    }
}

struct AggregationFuture<A>
where
    A: Aggregator,
{
    future: A::AggregateFut,
    response_tx: oneshot::Sender<Result<(), A::Error>>,
}

impl<A> Future for Service<A>
where
    A: Aggregator + Unpin,
{
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        trace!("polling Service");

        let pin = self.get_mut();

        if let Poll::Ready(_) = pin.poll_requests(cx) {
            return Poll::Ready(());
        }

        pin.poll_aggregation(cx);

        Poll::Pending
    }
}

pub struct ServiceRequests<A>(Pin<Box<dyn Stream<Item = Request<A>> + Send>>)
where
    A: Aggregator;

impl<A> Stream for ServiceRequests<A>
where
    A: Aggregator,
{
    type Item = Request<A>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        trace!("polling ServiceRequests");
        self.0.as_mut().poll_next(cx)
    }
}

impl<A> ServiceRequests<A>
where
    A: Aggregator + 'static,
{
    fn new(
        upload: UnboundedReceiver<UploadRequest>,
        download: UnboundedReceiver<DownloadRequest>,
        aggregate: UnboundedReceiver<AggregateRequest<A>>,
        select: UnboundedReceiver<SelectRequest<A>>,
    ) -> Self {
        let stream = download
            .map(Request::from)
            .merge(upload.map(Request::from))
            .merge(aggregate.map(Request::from))
            .merge(select.map(Request::from));
        Self(Box::pin(stream))
    }
}

#[derive(From)]
pub struct UploadRequest {
    credentials: Credentials,
    data: Bytes,
}

#[derive(From)]
pub struct DownloadRequest {
    credentials: Credentials,
    response_tx: oneshot::Sender<Result<Bytes, DownloadError>>,
}

#[derive(From)]
pub struct AggregateRequest<A>
where
    A: Aggregator,
{
    response_tx: oneshot::Sender<Result<(), A::Error>>,
}

#[derive(From)]
pub struct SelectRequest<A>
where
    A: Aggregator,
{
    credentials: Credentials,
    response_tx: oneshot::Sender<Result<(), A::Error>>,
}

#[derive(From)]
pub enum Request<A>
where
    A: Aggregator,
{
    Upload(UploadRequest),
    Download(DownloadRequest),
    Aggregate(AggregateRequest<A>),
    Select(SelectRequest<A>),
}

pub struct ServiceHandle<A>
where
    A: Aggregator,
{
    upload: UnboundedSender<UploadRequest>,
    download: UnboundedSender<DownloadRequest>,
    aggregate: UnboundedSender<AggregateRequest<A>>,
    select: UnboundedSender<SelectRequest<A>>,
}

// We implement Clone manually because it can only be derived if A:
// Clone, which we don't want.
impl<A> Clone for ServiceHandle<A>
where
    A: Aggregator,
{
    fn clone(&self) -> Self {
        Self {
            upload: self.upload.clone(),
            download: self.download.clone(),
            aggregate: self.aggregate.clone(),
            select: self.select.clone(),
        }
    }
}

impl<A> ServiceHandle<A>
where
    A: Aggregator + 'static,
{
    pub fn new() -> (Self, ServiceRequests<A>) {
        let (upload_tx, upload_rx) = unbounded_channel::<UploadRequest>();
        let (download_tx, download_rx) = unbounded_channel::<DownloadRequest>();
        let (aggregate_tx, aggregate_rx) = unbounded_channel::<AggregateRequest<A>>();
        let (select_tx, select_rx) = unbounded_channel::<SelectRequest<A>>();

        let handle = Self {
            upload: upload_tx,
            download: download_tx,
            aggregate: aggregate_tx,
            select: select_tx,
        };
        let service_requests =
            ServiceRequests::new(upload_rx, download_rx, aggregate_rx, select_rx);
        (handle, service_requests)
    }
    pub async fn download(
        &self,
        credentials: Credentials,
    ) -> Result<Bytes, ServiceError<DownloadError>> {
        let (tx, rx) = oneshot::channel::<Result<Bytes, DownloadError>>();
        let request = DownloadRequest::from((credentials, tx));
        Self::send_request(request, &self.download)?;
        Self::recv_response(rx)
            .await?
            .map_err(ServiceError::Request)
    }

    pub async fn upload(
        &self,
        credentials: Credentials,
        data: Bytes,
    ) -> Result<(), ServiceError<UploadError>> {
        let request = UploadRequest::from((credentials, data));
        Self::send_request(request, &self.upload)?;
        Ok(())
    }

    pub async fn aggregate(&self) -> Result<(), ServiceError<A::Error>> {
        let (tx, rx) = oneshot::channel::<Result<(), A::Error>>();
        Self::send_request(AggregateRequest::from(tx), &self.aggregate)?;
        Self::recv_response(rx)
            .await?
            .map_err(ServiceError::Request)
    }

    pub async fn select(&self, credentials: Credentials) -> Result<(), ServiceError<A::Error>> {
        let (tx, rx) = oneshot::channel::<Result<(), A::Error>>();
        Self::send_request(SelectRequest::from((credentials, tx)), &self.select)?;
        Self::recv_response(rx)
            .await?
            .map_err(ServiceError::Request)
    }

    fn send_request<P>(payload: P, tx: &UnboundedSender<P>) -> Result<(), ChannelError> {
        trace!("send request to the service");
        if tx.send(payload).is_err() {
            warn!("failed to send request: channel closed");
            Err(ChannelError::Request)
        } else {
            trace!("request sent");
            Ok(())
        }
    }

    async fn recv_response<R>(rx: oneshot::Receiver<R>) -> Result<R, ChannelError> {
        rx.await.map_err(|_| {
            warn!("could not receive response: channel closed");
            ChannelError::Response
        })
    }
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("the user does not have the proper permissions")]
    Unauthorized,
}

#[derive(Error, Debug)]
pub enum UploadError {
    #[error("the user does not have the proper permissions")]
    Unauthorized,
}

#[derive(Error, Debug)]
pub enum ServiceError<E>
where
    E: Error,
{
    #[error("failed to send the request or receive the response")]
    Handle(#[from] ChannelError),

    #[error("request failed: {0}")]
    Request(E),
}

#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("failed to send request to Service")]
    Request,
    #[error("failed to receive the response from Service")]
    Response,
}
