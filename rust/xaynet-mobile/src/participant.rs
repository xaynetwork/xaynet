use std::{convert::TryInto, sync::Arc};

use thiserror::Error;
use tokio::{
    runtime::Runtime,
    sync::{mpsc, Mutex},
};
use xaynet_core::mask::Model;
use xaynet_sdk::{
    client::{Client, ClientError},
    ModelStore,
    Notify,
    SerializableState,
    StateMachine,
    TransitionOutcome,
};

use crate::settings::{Settings, SettingsError};

pub enum Event {
    Update,
    Sum,
    Idle,
    NewRound,
    LoadModel,
}

pub struct Notifier(mpsc::Sender<Event>);
impl Notifier {
    fn notify(&mut self, event: Event) {
        if let Err(e) = self.0.try_send(event) {
            warn!("failed to notify participant: {}", e);
        }
    }
}

pub struct Events(mpsc::Receiver<Event>);

impl Events {
    fn new() -> (Self, Notifier) {
        let (tx, rx) = mpsc::channel(10);
        (Self(rx), Notifier(tx))
    }

    fn next(&mut self) -> Option<Event> {
        match self.0.try_recv() {
            Ok(event) => Some(event),
            Err(mpsc::error::TryRecvError::Empty) => None,
            // This can happen if:
            //  1. the state machine crashed. In that case it's OK to crash.
            //  2. `next` was called whereas the state machine was
            //     dropped, which is an error. So crashing is OK as
            //     well.
            Err(mpsc::error::TryRecvError::Closed) => panic!("notifier dropped"),
        }
    }
}

impl Notify for Notifier {
    fn new_round(&mut self) {
        self.notify(Event::NewRound)
    }
    fn sum(&mut self) {
        self.notify(Event::Sum)
    }
    fn update(&mut self) {
        self.notify(Event::Update)
    }
    fn load_model(&mut self) {
        self.notify(Event::LoadModel)
    }
    fn idle(&mut self) {
        self.notify(Event::Idle)
    }
}

#[derive(Clone)]
struct Store(Arc<Mutex<Option<Model>>>);

impl Store {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

#[async_trait]
impl ModelStore for Store {
    type Model = Model;
    type Error = std::convert::Infallible;

    async fn load_model(&mut self) -> Result<Option<Self::Model>, Self::Error> {
        Ok(self.0.lock().await.take())
    }
}

#[derive(Clone, Debug, Copy)]
pub enum Task {
    Sum,
    Update,
    None,
}

pub struct Participant {
    state_machine: Option<StateMachine>,
    events: Events,
    store: Store,
    runtime: Runtime,
    made_progress: bool,
    should_set_model: bool,
    task: Task,
}

#[derive(Error, Debug)]
pub enum InitError {
    #[error("failed to deserialize the participant state {:?}", _0)]
    Deserialization(Box<bincode::ErrorKind>),
    #[error("failed to initialize the participant runtime {:?}", _0)]
    Runtime(std::io::Error),
    #[error("failed to initialize HTTP client {:?}", _0)]
    Client(ClientError),
    #[error("invalid participant settings {:?}", _0)]
    InvalidSettings(#[from] SettingsError),
}

impl Participant {
    pub fn new(settings: Settings) -> Result<Self, InitError> {
        let (url, pet_settings) = settings.try_into().map_err(InitError::InvalidSettings)?;
        let client = Client::new(url, None, None).map_err(InitError::Client)?;
        let (events, notifier) = Events::new();
        let store = Store::new();
        let state_machine = StateMachine::new(pet_settings, client, store.clone(), notifier);
        Self::init(state_machine, events, store)
    }

    pub fn restore(state: &[u8], url: String) -> Result<Self, InitError> {
        let state: SerializableState =
            bincode::deserialize(state).map_err(InitError::Deserialization)?;
        let (events, notifier) = Events::new();
        let store = Store::new();
        let client = Client::new(url, None, None).map_err(InitError::Client)?;
        let state_machine = StateMachine::restore(state, client, store.clone(), notifier);
        Self::init(state_machine, events, store)
    }

    fn init(state_machine: StateMachine, events: Events, store: Store) -> Result<Self, InitError> {
        let mut participant = Self {
            runtime: Self::runtime()?,
            state_machine: Some(state_machine),
            events,
            store,
            task: Task::None,
            made_progress: true,
            should_set_model: false,
        };
        participant.process_events();
        Ok(participant)
    }

    fn runtime() -> Result<Runtime, InitError> {
        tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .map_err(InitError::Runtime)
    }

    pub fn save(self) -> Vec<u8> {
        // UNWRAP_SAFE: the state machine is always set.
        let state_machine = self.state_machine.unwrap().save();
        bincode::serialize(&state_machine).unwrap()
    }

    pub fn tick(&mut self) {
        // UNWRAP_SAFE: the state machine is always set.
        let state_machine = self.state_machine.take().unwrap();
        let outcome = self
            .runtime
            .block_on(async { state_machine.transition().await });
        match outcome {
            TransitionOutcome::Pending(new_state_machine) => {
                self.made_progress = false;
                self.state_machine = Some(new_state_machine);
            }
            TransitionOutcome::Complete(new_state_machine) => {
                self.made_progress = true;
                self.state_machine = Some(new_state_machine)
            }
        };
        self.process_events();
    }

    fn process_events(&mut self) {
        loop {
            match self.events.next() {
                Some(Event::Idle) => {
                    self.task = Task::None;
                }
                Some(Event::Update) => {
                    self.task = Task::Update;
                }
                Some(Event::Sum) => {
                    self.task = Task::Sum;
                }
                // not sure whether we need to do anything here
                Some(Event::NewRound) => {}
                Some(Event::LoadModel) => {
                    self.should_set_model = true;
                }
                None => break,
            }
        }
    }

    pub fn made_progress(&self) -> bool {
        self.made_progress
    }

    pub fn should_set_model(&self) -> bool {
        self.should_set_model
    }

    pub fn task(&self) -> Task {
        self.task
    }

    pub fn set_model(&mut self, model: Model) {
        let Self {
            ref mut runtime,
            ref store,
            ..
        } = self;

        runtime.block_on(async {
            let mut stored_model = store.0.lock().await;
            *stored_model = Some(model)
        });
        self.should_set_model = false;
    }
}
