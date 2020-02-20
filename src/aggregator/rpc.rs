use crate::common::{ClientId, Token};
use futures::{
    future::TryFutureExt,
    stream::{Stream, StreamExt},
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tarpc::{
    rpc::server::{BaseChannel, Channel},
    serde_transport::tcp::listen,
};

use tokio::sync::{mpsc, oneshot};
use tokio_serde::formats::Json;

#[tarpc::service]
/// Definition of the methods exposed by the aggregator RPC service.
pub trait RpcService {
    /// Notify the aggregator that the given client has been selected
    /// and should use the given token to download the global weights
    /// and upload their local weights.
    async fn select(id: ClientId, token: Token) -> Result<(), ()>;

    /// Notify the aggregator that it should clear its pool of client
    /// IDs and tokens. This should be called before starting a new
    /// round.
    async fn reset() -> Result<(), ()>;
}

// NOTE: the server is cloned on every request, so cloning should
// remain cheap!
#[derive(Clone)]
struct RpcServer {
    select: mpsc::UnboundedSender<RpcSelectRequest>,
    reset: mpsc::UnboundedSender<RpcResetRequest>,
}

impl RpcServer {
    fn new() -> (Self, RpcHandle) {
        let (select_tx, select_rx) = mpsc::unbounded_channel::<RpcSelectRequest>();
        let (reset_tx, reset_rx) = mpsc::unbounded_channel::<RpcResetRequest>();

        let server = RpcServer {
            select: select_tx,
            reset: reset_tx,
        };

        let handle = RpcHandle::new(select_rx, reset_rx);

        (server, handle)
    }
}

/// An incoming [`RpcService::select`] RPC request
pub type RpcSelectRequest = ((ClientId, Token), oneshot::Sender<()>);
/// An incoming [`RpcService::reset`] RPC request
pub type RpcResetRequest = oneshot::Sender<()>;

/// An incoming RPC request
pub enum RpcRequest {
    /// An incoming [`RpcService::select`] RPC request
    Select(RpcSelectRequest),
    /// An incoming [`RpcService::reset`] RPC request
    Reset(RpcResetRequest),
}

/// A handle to receive the RPC requests received by the RPC
/// [`RpcService`].
pub struct RpcHandle(Pin<Box<dyn Stream<Item = RpcRequest>>>);

impl RpcHandle {
    fn new(
        select: mpsc::UnboundedReceiver<RpcSelectRequest>,
        reset: mpsc::UnboundedReceiver<RpcResetRequest>,
    ) -> Self {
        Self(Box::pin(
            reset.map(RpcRequest::Reset).chain(select.map(RpcRequest::Select)),
        ))
    }
}

impl Stream for RpcHandle {
    type Item = RpcRequest;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.0.as_mut().poll_next(cx)
    }
}

impl RpcService for RpcServer {
    type SelectFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;
    type ResetFut = Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>;

    fn select(self, _: tarpc::context::Context, id: ClientId, token: Token) -> Self::SelectFut {
        let (tx, rx) = oneshot::channel();
        Box::pin(async move {
            self.select.send(((id, token), tx)).map_err(|_| ())?;
            rx.map_err(|_| ()).await
        })
    }

    fn reset(self, _: tarpc::context::Context) -> Self::ResetFut {
        let (tx, rx) = oneshot::channel();
        Box::pin(async move {
            self.reset.send(tx).map_err(|_| ())?;
            rx.map_err(|_| ()).await
        })
    }
}

/// Start an RPC server that accepts only one connection at a time.
async fn run_rpc(mut rpc_handle_tx: mpsc::Sender<RpcHandle>) -> ::std::io::Result<()> {
    // FIXME: this should obviously be configurable
    let listen_addr = "127.0.0.1:50052";
    let mut listener = listen(listen_addr, Json::default).await?;

    while let Some(accept_result) = listener.next().await {
        match accept_result {
            Ok(transport) => {
                let channel = BaseChannel::with_defaults(transport);
                let (server, handle) = RpcServer::new();
                if rpc_handle_tx.send(handle).await.is_err() {
                    continue;
                }
                let handler = channel.respond_with(server.serve());
                handler.execute().await;
            }
            Err(e) => error!("failed to accept RPC connection: {:?}", e),
        }
    }
    Ok(())
}
