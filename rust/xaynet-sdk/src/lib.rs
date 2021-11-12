#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
    doc,
    forbid(rustdoc::broken_intra_doc_links, rustdoc::private_intra_doc_links)
)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/xaynetwork/xaynet/master/assets/xaynet_banner.png",
    html_favicon_url = "https://raw.githubusercontent.com/xaynetwork/xaynet/master/assets/favicon.png",
    issue_tracker_base_url = "https://github.com/xaynetwork/xaynet/issues"
)]
//! This crate provides building blocks for implementing participants for
//! the [Xaynet Federated Learning platform](https://www.xaynet.dev/).
//!
//! The PET protocol states that in any given round of federated learning,
//! each participant of the protocol may be selected to carry out one of
//! two tasks:
//!
//! - **update**: participants selected for the update task
//!   (a.k.a. _update participants_) are responsible for sending a machine
//!   learning model they trained
//! - **sum**: participants selected for the sum task (a.k.a. _sum
//!   participants_) are responsible for computing a global mask from local mask seeds sent by
//!   the update participants
//!
//! Participants may also not be selected for any of these tasks, in which
//! case they simply wait for the next round.
//!
//! # Running a participant
//!
//! The communication with the Xaynet coordinator is managed by a
//! background task that runs the PET protocol. We call it the PET
//! agent. In practice, the agent is a simple wrapper around the
//! [`StateMachine`].
//!
//! To run a participant, you need to start an agent, and
//! interact with it. There are two types of interactions:
//!
//! - reacting to notifications for the agents, which include:
//!   - start of a new round of training
//!   - selection for the sum task
//!   - selection for the update task
//!   - end of a task
//! - providing the agent with a Machine Learning model and a corresponding
//!   scalar for aggregation when the participant takes part the update task
//!
//! ## Implementing an agent
//!
//! A simple agent can be implemented as a function.
//!
//! ```
//! use std::time::Duration;
//!
//! use tokio::time::sleep;
//! use xaynet_sdk::{StateMachine, TransitionOutcome};
//!
//! async fn run_agent(mut state_machine: StateMachine, tick: Duration) {
//!     loop {
//!         state_machine = match state_machine.transition().await {
//!             // The state machine is stuck waiting for some data,
//!             // either from the coordinator or from the
//!             // participant. Let's wait a little and try again
//!             TransitionOutcome::Pending(state_machine) => {
//!                 sleep(tick).await;
//!                 state_machine
//!             }
//!             // The state machine moved forward in the PET protocol.
//!             // We simply continue looping, trying to make more progress.
//!             TransitionOutcome::Complete(state_machine) => state_machine,
//!         };
//!     }
//! }
//! ```
//!
//! This agent needs to be fed a [`StateMachine`] in order to run. A
//! state machine requires found components:
//!
//! - a cryptographic key identifying the participant, see [`PetSettings`]
//! - a store from which it can load a model when the participant is
//!   selected for the update task. This can be any type that
//!   implements the [`ModelStore`] trait. In our case, we'll use a
//!   dummy in-memory store that always returns the same model.
//! - a client to talk with the Xaynet coordinator. This can be any
//!   type that implements the [`XaynetClient`] trait, like the [`Client`].
//!   For this we're going to use the trait implementations on the `reqwest`
//!   client that is available when compiling with `--features reqwest-client`.
//! - a notifier that the state machine can use to send
//!   notifications. This can be any type that implements the
//!   [`Notify`] trait. We'll use channels for this.
//!
//! [`PetSettings`]: crate::settings::PetSettings
//! [`Client`]: crate::client::Client
//!
//! Finally we can start our agent and log the events it emits. Here
//! is the full code:
//!
//! ```no_run
//! # #[cfg(all(feature = "reqwest-client", feature = "tokio/rt-muli-thread"))]
//! # mod feature_reqwest_client {
//! use std::{
//!     sync::{mpsc, Arc},
//!     time::Duration,
//! };
//!
//! use async_trait::async_trait;
//! use reqwest::Client as ReqwestClient;
//! use tokio::time::sleep;
//!
//! use xaynet_core::{
//!     crypto::SigningKeyPair,
//!     mask::{BoundType, DataType, FromPrimitives, GroupType, MaskConfig, Model, ModelType},
//! };
//! use xaynet_sdk::{
//!     client::Client,
//!     settings::PetSettings,
//!     ModelStore,
//!     Notify,
//!     StateMachine,
//!     TransitionOutcome,
//! };
//!
//! async fn run_agent(mut state_machine: StateMachine, tick: Duration) {
//!     loop {
//!         state_machine = match state_machine.transition().await {
//!             TransitionOutcome::Pending(state_machine) => {
//!                 sleep(tick.clone()).await;
//!                 state_machine
//!             }
//!             TransitionOutcome::Complete(state_machine) => state_machine,
//!         };
//!     }
//! }
//!
//! #[derive(Debug)]
//! enum Event {
//!     // event sent by the state machine when the participant is
//!     // selected for the update task
//!     Update,
//!     // event sent by the state machine when the participant is
//!     // selected for the sum task
//!     Sum,
//!     // event sent by the state machine when a new round starts
//!     NewRound,
//!     // event sent by the state machine when the participant
//!     // becomes inactive (after finishing a task for instance)
//!     Idle,
//!     // event sent by the state machine when the participant
//!     // is supposed to populate the model store
//!     LoadModel,
//! }
//!
//! // Our notifier is a simple wrapper around a channel.
//! struct Notifier(mpsc::Sender<Event>);
//!
//! impl Notify for Notifier {
//!     fn new_round(&mut self) {
//!         self.0.send(Event::NewRound).unwrap();
//!     }
//!     fn sum(&mut self) {
//!         self.0.send(Event::Sum).unwrap();
//!     }
//!     fn update(&mut self) {
//!         self.0.send(Event::Update).unwrap();
//!     }
//!     fn idle(&mut self) {
//!         self.0.send(Event::Idle).unwrap();
//!     }
//!     fn load_model(&mut self) {
//!         self.0.send(Event::LoadModel).unwrap();
//!     }
//! }
//!
//! // Our store will always load the same model.
//! // In practice the model should be updated with
//! // the model the participant trains when it is selected
//! // for the update task.
//! struct LocalModel(Arc<Model>);
//!
//! #[async_trait]
//! impl ModelStore for LocalModel {
//!     type Model = Arc<Model>;
//!     type Error = std::convert::Infallible;
//!
//!     async fn load_model(&mut self) -> Result<Option<Self::Model>, Self::Error> {
//!         Ok(Some(self.0.clone()))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), std::convert::Infallible> {
//!     let keys = SigningKeyPair::generate();
//!     let settings = PetSettings::new(keys);
//!     let xaynet_client = Client::new(ReqwestClient::new(), "http://localhost:8081").unwrap();
//!     let (tx, rx) = mpsc::channel::<Event>();
//!     let notifier = Notifier(tx);
//!     let model = Model::from_primitives(vec![0; 100].into_iter()).unwrap();
//!     let model_store = LocalModel(Arc::new(model));
//!
//!     let mut state_machine = StateMachine::new(settings, xaynet_client, model_store, notifier);
//!     // Start the agent
//!     tokio::spawn(async move {
//!         run_agent(state_machine, Duration::from_secs(1)).await;
//!     });
//!
//!     loop {
//!         println!("{:?}", rx.recv().unwrap());
//!     }
//! }
//! # }
//! # fn main() {} // don't actually run anything, because the client never terminates
//! ```

pub mod client;
mod message_encoder;
pub mod settings;
mod state_machine;
mod traits;
pub(crate) mod utils;

pub(crate) use self::message_encoder::MessageEncoder;
pub use self::traits::{ModelStore, Notify, XaynetClient};
pub use state_machine::{LocalModelConfig, SerializableState, StateMachine, TransitionOutcome};
