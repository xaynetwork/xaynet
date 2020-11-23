use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use tokio::{sync::mpsc, time::delay_for};
use tracing::{info, warn};

use xaynet_core::mask::Model;
use xaynet_sdk::{
    client::Client,
    settings::PetSettings,
    ModelStore,
    Notify,
    StateMachine,
    TransitionOutcome,
    XaynetClient,
};

enum Event {
    Update,
    Sum,
    NewRound,
    Idle,
}

pub struct Participant {
    // FIXME: XaynetClient requires the client to be mutable. This may
    // make it easier to implement clients, but as a result we can't
    // wrap the client in an Arc, which would allow us to share the
    // same client with all the participants. Maybe XaynetClient
    // should have methods that take &self?
    xaynet_client: Client<reqwest::Client>,
    notifications: mpsc::Receiver<Event>,
}

pub struct Agent(StateMachine);

impl Agent {
    fn new<X, M, N>(settings: PetSettings, xaynet_client: X, model_store: M, notify: N) -> Self
    where
        X: XaynetClient + Send + 'static,
        M: ModelStore + Send + 'static,
        N: Notify + Send + 'static,
    {
        Agent(StateMachine::new(
            settings,
            xaynet_client,
            model_store,
            notify,
        ))
    }

    pub async fn run(mut self, tick: Duration) {
        loop {
            self = match self.0.transition().await {
                TransitionOutcome::Pending(state_machine) => {
                    delay_for(tick).await;
                    Self(state_machine)
                }
                TransitionOutcome::Complete(state_machine) => Self(state_machine),
            };
        }
    }
}

impl Participant {
    pub fn new(
        settings: PetSettings,
        xaynet_client: Client<reqwest::Client>,
        model: Arc<Model>,
    ) -> (Self, Agent) {
        let (tx, rx) = mpsc::channel::<Event>(10);
        let notifier = Notifier(tx);
        let agent = Agent::new(settings, xaynet_client.clone(), LocalModel(model), notifier);
        let participant = Self {
            xaynet_client,
            notifications: rx,
        };
        (participant, agent)
    }

    pub async fn run(mut self) {
        use Event::*;
        loop {
            match self.notifications.recv().await {
                Some(Sum) => {
                    info!("taking part in the sum task");
                }
                Some(Update) => {
                    info!("taking part to the update task");
                }
                Some(Idle) => {
                    info!("waiting");
                }
                Some(NewRound) => {
                    info!("new round started, downloading latest global model");
                    if let Err(e) = self.xaynet_client.get_model().await {
                        warn!("failed to download latest model: {}", e);
                    }
                }
                None => {
                    warn!("notifications channel closed, terminating");
                    return;
                }
            }
        }
    }
}

struct Notifier(mpsc::Sender<Event>);

impl Notify for Notifier {
    fn new_round(&mut self) {
        if let Err(e) = self.0.try_send(Event::NewRound) {
            warn!("failed to notify participant: {}", e);
        }
    }

    fn sum(&mut self) {
        if let Err(e) = self.0.try_send(Event::Sum) {
            warn!("failed to notify participant: {}", e);
        }
    }

    fn update(&mut self) {
        if let Err(e) = self.0.try_send(Event::Update) {
            warn!("failed to notify participant: {}", e);
        }
    }

    fn idle(&mut self) {
        if let Err(e) = self.0.try_send(Event::Idle) {
            warn!("failed to notify participant: {}", e);
        }
    }
}

pub struct LocalModel(Arc<Model>);

#[async_trait]
impl ModelStore for LocalModel {
    type Model = Arc<Model>;
    type Error = std::convert::Infallible;

    async fn load_model(&mut self) -> Result<Option<Self::Model>, Self::Error> {
        Ok(Some(self.0.clone()))
    }
}
