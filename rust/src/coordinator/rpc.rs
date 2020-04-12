use crate::{
    common::{
        client::ClientId,
        sync::{SendReset, SyncHandle, SyncRequest},
    },
    coordinator::core::ServiceHandle,
};
use async_trait::async_trait;
use std::{future::Future, io, iter, pin::Pin, time::Duration};
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tarpc::{
    client::Config,
    context::Context,
    rpc::server::{BaseChannel, Channel},
    serde_transport::{tcp::listen, Transport},
};
use tokio::{net::ToSocketAddrs, stream::StreamExt};
use tokio_serde::formats::Json;
use tracing_futures::Instrument;

mod inner {
    use crate::common::client::ClientId;
    #[tarpc::service]
    pub trait Rpc {
        async fn end_training(id: ClientId, success: bool);

        async fn sync();
    }
}

#[async_trait]
impl SendReset for inner::RpcClient {
    async fn reset(&mut self, ctx: Context) -> std::io::Result<()> {
        self.sync(ctx).await
    }
}

pub use inner::Rpc;

#[cfg(test)]
pub use crate::tests::lib::rpc::coordinator::Client;
#[cfg(not(test))]
pub use inner::RpcClient as Client;

impl Rpc for Server {
    type EndTrainingFut = Pin<Box<dyn Future<Output = ()> + Send>>;
    type SyncFut = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn end_training(
        self,
        _: tarpc::context::Context,
        id: ClientId,
        success: bool,
    ) -> Self::EndTrainingFut {
        debug!("handling end training request");
        let span = trace_span!("rpc_end_training_handler", client_id = %id, success = &success);
        Box::pin(async move { self.0.end_training(id, success).await }.instrument(span))
    }

    fn sync(self, _: tarpc::context::Context) -> Self::SyncFut {
        debug!("handling reset request");
        let span = trace_span!("rpc_reset_handler");
        Box::pin(
            async move {
                let _ = self.1.sync(SyncRequest::ExternalRequest).await;
            }
            .instrument(span),
        )
    }
}

/// A server that serves a single client. A new `Server` is created
/// for each new client.
#[derive(Clone)]
struct Server(ServiceHandle, SyncHandle);

pub async fn client_connect<A: ToSocketAddrs + Unpin + Clone + Send + Sync + 'static>(
    addr: A,
    on_disconnect: impl Fn() + 'static + Send + Sync,
) -> io::Result<Client> {
    let reconnect_opts = ReconnectOptions::new()
        .with_exit_if_first_connect_fails(false)
        .with_retries_generator(|| iter::repeat(Duration::from_secs(1)))
        .with_on_disconnect_callback(on_disconnect);
    let tcp_stream = StubbornTcpStream::connect_with_options(addr, reconnect_opts).await?;
    let transport = Transport::from((tcp_stream, Json::default()));
    Client::new(Config::default(), transport).spawn()
}

/// Run an RPC server that processes only one connection at a time.
pub async fn serve<A: ToSocketAddrs + Send + Sync + 'static>(
    addr: A,
    service_handle: ServiceHandle,
    sync_handle: SyncHandle,
) -> ::std::io::Result<()> {
    let mut listener = listen(addr, Json::default).await?;

    while let Some(accept_result) = listener.next().await {
        match accept_result {
            Ok(transport) => {
                let channel = BaseChannel::with_defaults(transport);
                let server = Server(service_handle.clone(), sync_handle.clone());
                let handler = channel.respond_with(server.serve());
                handler
                    .execute()
                    // FIXME: add peer to span
                    .instrument(trace_span!("rpc_handler"))
                    .await;
            }
            Err(e) => error!("failed to accept RPC connection: {:?}", e),
        }
    }
    Ok(())
}
