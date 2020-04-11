use async_trait::async_trait;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[async_trait]
pub trait ProcessSync {
    async fn reset(&self, req: SyncRequest);
}

#[derive(Debug)]
pub enum SyncRequest {
    // The RPC server received a sync request from the aggregator.
    External,
    // The RPC client lost the connection to the aggregator RPC server.
    Internal,
}

pub struct SyncHandle<P>
where
    P: ProcessSync,
{
    service_handle: P,
    sync_rx: UnboundedReceiver<SyncRequest>,
}

impl<P> SyncHandle<P>
where
    P: ProcessSync,
{
    pub fn new(service_handle: P) -> (Self, UnboundedSender<SyncRequest>) {
        let (sync_tx, sync_rx) = unbounded_channel::<SyncRequest>();
        (
            Self {
                service_handle,
                sync_rx,
            },
            sync_tx,
        )
    }

    async fn sync(&self, req: SyncRequest) {
        self.service_handle.reset(req).await;
    }

    fn get_sync_rx(&mut self) -> &mut UnboundedReceiver<SyncRequest> {
        &mut self.sync_rx
    }
}

pub async fn run_sync_handle<P>(mut sync_handle: SyncHandle<P>)
where
    P: ProcessSync,
{
    loop {
        match sync_handle.get_sync_rx().recv().await {
            Some(req) => {
                debug!("Received {:?} sync request", &req);
                sync_handle.sync(req).await;
            }
            None => {
                warn!("All senders have been dropped!");
                return;
            }
        }
    }
}
