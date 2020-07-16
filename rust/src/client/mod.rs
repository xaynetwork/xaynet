//! Provides client-side functionality to connect to a XayNet service.
//!
//! This functionality includes:
//!
//! * Abiding by (the underlying [`Participant`]'s side of) the PET protocol.
//! * Handling the network communication with the XayNet service, including
//!   polling of service data.
//!
//! # Participant
//! In any given round of federated learning, each [`Participant`] of the
//! protocol is characterised by a role which determines its [`Task`] to carry
//! out in the round, and which is computed by [`check_task`].
//!
//! Participants selected to `Update` are responsible for sending masked model
//! updates in the form of PET messages constructed with
//! [`compose_update_message`].
//!
//! Participants selected to `Sum` are responsible for sending ephemeral keys
//! and global masks in PET messages constructed respectively with
//! [`compose_sum_message`] and [`compose_sum2_message`].
//!
//! # Client
//! A [`Client`] has an intentionally simple API - the idea is that it is
//! initialised with some settings, and then [`start()`]ed. Currently for
//! simplicity, clients that have started running will do so indefinitely. It is
//! therefore the user's responsibility to terminate clients that are no longer
//! needed. Alternatively, it may be more convenient to run just a single round
//! (or a known fixed number of rounds). In this case, use [`during_round()`].
//! For examples of usage, see the `test-drive` scripts.
//!
//! **Note.** At present, the [`Client`] implementation is somewhat tightly
//! coupled with the workings of the C-API SDK, but this may well change in a
//! future version to be more independently reusable.
//!
//! ## Requests via Proxy
//! There is a [`Proxy`] which a [`Client`] can use to communicate with the
//! service. To summarise, the proxy:
//!
//! * Wraps either an in-memory service (for local comms) or a _client request_
//! object (for remote comms over HTTP).
//! * In the latter case, deals with logging and wrapping of network errors.
//! * Deals with deserialization
//!
//! The client request object is responsible for building the HTTP request and
//! extracting the response body. As an example:
//!
//! ```no_rust
//! async fn get_sums(&self) -> Result<Option<bytes::Bytes>, reqwest::Error>
//! ```
//!
//! issues a GET request for the sum dictionary. The return type reflects the
//! presence of networking `Error`s, but also the situation where the dictionary
//! is simply just not yet available on the service. That is, the type also
//! reflects the _optionality_ of the data availability.
//!
//! [`Proxy`] essentially takes this (deserializing the `Bytes` into a `SumDict`
//! while handling `Error`s into [`ClientError`]s) to expose the overall method
//!
//! ```no_rust
//! async fn get_sums(&self) -> Result<Option<SumDict>, ClientError>
//! ```
//!
//! [`check_task`]: #method.check_task
//! [`compose_update_message`]: #method.compose_update_message
//! [`compose_sum_message`]: #method.compose_sum_message
//! [`compose_sum2_message`]: #method.compose_sum2_message
//! [`start()`]: #method.start
//! [`during_round()`]: #method.during_round

use crate::{
    mask::model::Model,
    services::{FetchError, PetMessageError},
    InitError,
    PetError,
};
use std::{future::Future, sync::Arc, thread};
use thiserror::Error;
use tokio::{
    runtime,
    sync::{broadcast, mpsc, watch},
};

pub mod mobile_client;

mod client;
pub use client::{Client, RoundParamFetcher};

mod request;
pub use request::Proxy;

mod participant;
pub use participant::{Participant, Task};

#[derive(Debug, Error)]
/// Client-side errors
pub enum ClientError {
    #[error("failed to initialise participant: {0}")]
    /// Failed to initialise participant.
    ParticipantInitErr(InitError),

    #[error("Failed to retrieve data: {0}")]
    /// Failed to retrieve data.
    Fetch(FetchError),

    #[error("Failed to handle PET message: {0}")]
    /// Failed to handle PET message.
    PetMessage(PetMessageError),

    #[error("error arising from participant")]
    /// Error arising from participant.
    ParticipantErr(PetError),

    #[error("failed to deserialise service data: {0}")]
    /// Failed to deserialise service data.
    DeserialiseErr(bincode::Error),

    #[error("network-related error: {0}")]
    /// Network-related error.
    NetworkErr(reqwest::Error),

    #[error("failed to parse service data")]
    /// Failed to parse service data.
    ParseErr,

    #[error("unexpected client error")]
    /// Unexpected client error.
    GeneralErr,

    #[error("mobile client failed: {0}")]
    MobileClientError(&'static str),
}

pub struct AsyncClient {
    /// Proxy for the service
    proxy: Arc<Proxy>,
    local_model: LocalModelCache,
    global_model: GlobalModelCache,
}

impl AsyncClient {
    pub fn new(addr: &str) -> Result<Self, ClientError> {
        Ok(Self {
            proxy: Arc::new(Proxy::new_remote(addr)),
            local_model: LocalModelCache::new(),
            global_model: GlobalModelCache::new(),
        })
    }

    pub fn set_local_model(&mut self, model: Model) {
        self.local_model.set_local_model(model);
    }
    pub fn get_global_model(&mut self) -> Option<Model> {
        self.global_model.get_latest()
    }

    // here we should track if we have already started a client
    pub fn start(&mut self, shutdown_rx: broadcast::Receiver<()>) -> impl Future<Output = ()> {
        let (global_model_tx, global_model_rx) = watch::channel(None);
        self.global_model.set_receiver(Some(global_model_rx));

        Client::start(
            self.proxy.clone(),
            self.local_model.get_receiver(),
            global_model_tx,
            shutdown_rx,
        )
    }
}

// A cache to store the local model.
// we want to store the local model even if the Client is not running.
// As soon as the client has started we can use the model during the update task
pub struct LocalModelCache {
    sender: watch::Sender<Option<Model>>,
    // The type of receiver should be: Arc<watch::Receiver<Option<Model>>>
    // to keep the behaviour of the recv method:
    // "If this is the first time the function is called on a Receiver instance, then the function
    // completes immediately with the current value held by the channel. On the next call,
    // the function waits until a new value is sent in the channel."
    // because we only want to use the local model once. So if the local model was used in round 1
    // we don't want to use the same local model in round 2 instead we want to wait for a new local
    // model .
    // However one issue remains: If the update task took the model but it is canceled afterwards,
    // the local model is lost.
    receiver: watch::Receiver<Option<Model>>,
}

impl LocalModelCache {
    pub fn new() -> Self {
        let (sender, receiver) = watch::channel(None);
        Self { sender, receiver }
    }

    pub fn set_local_model(&mut self, model: Model) {
        self.sender.broadcast(Some(model));
    }

    fn get_receiver(&mut self) -> watch::Receiver<Option<Model>> {
        self.receiver.clone()
    }
}

// A cache to store the global model.
// We want to be able to provide the latest fetch model even if the Client is not running
pub struct GlobalModelCache {
    current_model: Option<Model>,
    receiver: Option<watch::Receiver<Option<Model>>>,
}

impl GlobalModelCache {
    pub fn new() -> Self {
        Self {
            current_model: None,
            receiver: None,
        }
    }

    pub fn set_receiver(&mut self, receiver: Option<watch::Receiver<Option<Model>>>) {
        self.receiver = receiver
    }

    pub fn get_latest(&mut self) -> Option<Model> {
        if let Some(ref receiver) = self.receiver {
            self.current_model = receiver.borrow().clone();
            self.current_model.clone()
        } else {
            None
        }
    }
}

pub struct SyncClient {
    client: AsyncClient,
    handle: Option<thread::JoinHandle<()>>,
    shutdown: Option<broadcast::Sender<()>>,
}

impl SyncClient {
    pub fn new(addr: &str) -> Self {
        Self {
            client: AsyncClient::new(addr).unwrap(),
            handle: None,
            shutdown: None,
        }
    }

    pub fn start(&mut self) {
        if self.handle.is_none() {
            let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);
            let client = self.client.start(shutdown_rx);

            let handle = thread::spawn(move || {
                let mut runtime = runtime::Builder::new()
                    .threaded_scheduler()
                    .enable_all()
                    .build()
                    .unwrap();
                runtime.block_on(async move { client.await });
            });
            self.shutdown = Some(shutdown_tx);
            self.handle = Some(handle)
        }
    }

    pub fn get_global_model(&mut self) -> Option<Model> {
        self.client.get_global_model()
    }

    pub fn set_local_model(&mut self, local_model: Model) {
        self.client.set_local_model(local_model);
    }

    pub fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            // dropping the shutdown handle will trigger the shutdown tokio:select branch of
            // the RoundParamFetcher which will trigger the shutdown the participant task
            drop(shutdown);
            // we wait until the tokio runtime has finished the task
            self.handle.take().unwrap().join();
        }
    }
}
