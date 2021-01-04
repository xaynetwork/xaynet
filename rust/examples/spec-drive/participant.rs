use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use tokio::{sync::mpsc, time::delay_for};
use tracing::{info, warn};

use xaynet_core::mask::{Analytic, Model};
use xaynet_sdk::{
    client::Client,
    settings::PetSettings,
    ModelStore,
    Notify,
    Preprocessor,
    StateMachine,
    TransitionOutcome,
    XaynetClient,
};

enum Event {
    Update,
    Sum,
    NewRound,
    Idle,
    LoadModel,
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

pub struct Participant {
    // FIXME: XaynetClient requires the client to be mutable. This may
    // make it easier to implement clients, but as a result we can't
    // wrap the client in an Arc, which would allow us to share the
    // same client with all the participants. Maybe XaynetClient
    // should have methods that take &self?
    xaynet_client: Client<reqwest::Client>,
    notifications: mpsc::Receiver<Event>,
    data: LocalData,
    spec: Option<Analytic>,
    readings: Preprocessor,
}

impl Participant {
    pub fn new(
        settings: PetSettings,
        xaynet_client: Client<reqwest::Client>,
        readings: Preprocessor,
    ) -> (Self, Agent) {
        let (tx, rx) = mpsc::channel::<Event>(10);
        let notifier = Notifier(tx);
        let data = LocalData::new();
        let agent = Agent::new(settings, xaynet_client.clone(), data.clone(), notifier);
        let participant = Self {
            xaynet_client,
            notifications: rx,
            data,
            spec: None,
            readings,
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
                    info!("taking part in the update task");
                    if let Some(ref spec) = self.spec {
                        // TODO can't do much with unit right now...
                        let (vect, _unit) = self.readings.measure(spec);
                        let _prev = self.data.set(vect);
                    } else {
                        warn!("no global spec");
                    }
                }
                Some(Idle) => {
                    info!("waiting");
                }
                Some(NewRound) => {
                    info!("new round started, downloading latest global spec");
                    match self.xaynet_client.get_spec().await {
                        Err(e) => warn!("failed to download latest spec: {}", e),
                        Ok(spec) => self.spec = spec, // TODO retry on None?
                    }
                }
                Some(LoadModel) => {
                    info!("agent wants data to be set");
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

    fn load_model(&mut self) {
        if let Err(e) = self.0.try_send(Event::LoadModel) {
            warn!("failed to notify participant: {}", e);
        }
    }
}

#[derive(Clone)]
struct LocalData(Arc<Mutex<Option<Model>>>);

impl LocalData {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }

    fn take(&self) -> Option<Model> {
        let mut lock = self.0.lock().unwrap();
        lock.take()
    }

    /// Sets a new data item, returning the replaced one.
    fn set(&mut self, data: Model) -> Option<Model> {
        let mut lock = self.0.lock().unwrap();
        lock.replace(data)
    }
}

#[async_trait]
impl ModelStore for LocalData {
    type Model = Model;
    type Error = std::convert::Infallible;

    async fn load_model(&mut self) -> Result<Option<Self::Model>, Self::Error> {
        Ok(self.take())
    }
}
