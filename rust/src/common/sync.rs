use crate::common::state::StateHandle;
use async_trait::async_trait;
use futures::{ready, stream::Stream};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, SystemTime},
};
use tokio::{
    stream::StreamExt,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};

pub enum SyncTasks {
    Enter,
    Reset,
    Exit,
}

#[async_trait]
pub trait ProcessSync {
    async fn enter_sync(&self);
    async fn reset(&self);
    async fn exit_sync(&self);
}

#[async_trait]
pub trait SendReset {
    async fn reset(&mut self, ctx: tarpc::context::Context) -> std::io::Result<()>;
}

#[derive(Debug)]
pub enum SyncRequest {
    // The RPC server received a sync request from the aggregator.
    ExternalRequest,
    // The RPC client lost the connection to the aggregator RPC server.
    RPCClientDisconnect,
}

pub struct SyncService<S, R>
where
    S: ProcessSync + Clone + Send + Sync + Unpin + 'static,
    R: SendReset + Clone + Send + Sync + Unpin + 'static,
{
    service_handle: S,
    rpc_client: R,
    state_handle: StateHandle,
    sync_requests: SyncRequests,
}

impl<S, R> SyncService<S, R>
where
    S: ProcessSync + Clone + Send + Sync + Unpin + 'static,
    R: SendReset + Clone + Send + Sync + Unpin + 'static,
{
    pub fn new(
        service_handle: S,
        rpc_client: R,
        state_handle: StateHandle,
        sync_requests: SyncRequests,
    ) -> Self {
        Self {
            service_handle,
            rpc_client,
            state_handle,
            sync_requests,
        }
    }

    /// Handle the incoming requests.
    fn poll_requests(&mut self, cx: &mut Context) -> Poll<()> {
        trace!("polling requests");
        loop {
            match ready!(Pin::new(&mut self.sync_requests).poll_next(cx)) {
                Some(request) => {
                    self.handle_request(request);
                }
                None => return Poll::Ready(()),
            }
        }
    }

    /// Handle a request
    fn handle_request(&mut self, request: SyncRequest) {
        match request {
            SyncRequest::RPCClientDisconnect => self.handle_rpc_client_disconnect_request(),
            SyncRequest::ExternalRequest => self.handle_external_request(),
        }
    }

    /// Handle a rpc client disconnect request
    fn handle_rpc_client_disconnect_request(&mut self) {
        debug!("handle rpc_client_disconnect request");
        let service_handle = self.service_handle.clone();
        let mut rpc_client = self.rpc_client.clone();
        let _ = tokio::spawn(async move {
            service_handle.enter_sync().await;
            service_handle.reset().await;

            let mut context = tarpc::context::current();
            context.deadline = SystemTime::now() + Duration::from_secs(120);
            rpc_client
                .reset(context)
                .await
                .expect("Timeout. Could not connect to the external service");

            service_handle.exit_sync().await;
        });
    }

    /// Handle an external request
    fn handle_external_request(&mut self) {
        debug!("handle external sync request");
        let service_handle = self.service_handle.clone();
        let _ = tokio::spawn(async move {
            service_handle.enter_sync().await;
            service_handle.reset().await;
            service_handle.exit_sync().await;
        });
    }
}

impl<S, R> Future for SyncService<S, R>
where
    S: ProcessSync + Clone + Send + Sync + Unpin + 'static,
    R: SendReset + Clone + Send + Sync + Unpin + 'static,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        trace!("polling Service");
        let pin = self.get_mut();

        match pin.poll_requests(cx) {
            Poll::Ready(()) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct SyncRequests(Pin<Box<dyn Stream<Item = SyncRequest> + Send>>);

impl Stream for SyncRequests {
    type Item = SyncRequest;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        trace!("polling SyncRequest");
        self.0.as_mut().poll_next(cx)
    }
}

impl SyncRequests {
    fn new(sync: UnboundedReceiver<SyncRequest>) -> Self {
        Self(Box::pin(sync.map(SyncRequest::from)))
    }
}

#[derive(Clone)]
pub struct SyncHandle(UnboundedSender<SyncRequest>);

impl SyncHandle {
    pub fn new() -> (Self, SyncRequests) {
        let (sync_tx, sync_rx) = unbounded_channel::<SyncRequest>();

        let handle = Self(sync_tx);

        let state_request = SyncRequests::new(sync_rx);
        (handle, state_request)
    }

    pub async fn sync(&self, req: SyncRequest) {
        Self::send_request(SyncRequest::from(req), &self.0);
    }

    fn send_request<P>(payload: P, chan: &UnboundedSender<P>) {
        trace!("send request to the state service");
        if chan.send(payload).is_err() {
            warn!("failed to send request: channel closed");
            return;
        }
        trace!("request sent");
    }
}
