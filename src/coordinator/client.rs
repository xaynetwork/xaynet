use super::heartbeat::*;
use super::state_machine::*;
use derive_more::Display;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    mem,
    time::Duration,
};
use tokio::sync::mpsc;
use uuid::Uuid;

const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone, Display)]
/// A unique random client identifier
pub struct ClientId(Uuid);

impl ClientId {
    /// Return a new random client identifier
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Represent an active client.
struct ActiveClient {
    /// Channel for resetting this client's heartbeat timer
    heartbeat_reset: mpsc::Sender<Duration>,
}

impl ActiveClient {
    /// Create a new active client
    fn new(heartbeat_reset: mpsc::Sender<Duration>) -> Self {
        Self { heartbeat_reset }
    }

    /// Reset the client's heartbeat timer.
    fn reset_heartbeat(&mut self, timeout: Duration) -> Result<(), HeartBeatResetError> {
        self.heartbeat_reset.try_send(timeout).map_err(|err| {
            match err {
                mpsc::error::TrySendError::Full(_) => {
                    warn!("could not reset heartbeat timer: a client may be flooding us with heartbeat requests");
                    HeartBeatResetError::BackPressure
                }
                mpsc::error::TrySendError::Closed(_) => {
                    debug!("could not reset heartbeat timer: timer dropped already");
                    HeartBeatResetError::Expired
                }
            }
        })
    }
}

/// A store for all the clients the coordinator is tracking.
pub struct Clients {
    /// Active clients that are not selected for the current training
    /// round but could be selected at some point. It corresponds to
    /// clients in state [`ClientState::Waiting`],
    waiting: HashMap<ClientId, ActiveClient>,

    /// Active clients that are selected for the current training
    /// round, but haven't finish training. It corresponds to clients
    /// in state [`ClientState::Selected`], [`ClientState::Training`]
    selected: HashMap<ClientId, ActiveClient>,

    /// Active clients that cannot be selected for the current
    /// round. It corresponds to clients in state
    /// [`ClientState::Ignored`].
    ignored: HashMap<ClientId, ActiveClient>,

    /// Active clients that took part to the current training round
    /// and finished training. I corresponds to clients in state
    /// [`ClientState::Done`].
    done: HashMap<ClientId, ActiveClient>,

    /// Clients that were selected for the current training round and
    /// that finished training their model, but that are not active
    /// anymore. It corresponds to clients in state
    /// [`ClientState::DoneAndInactive`]
    done_and_inactive: HashSet<ClientId>,

    /// A channel that can be cloned. When instanciating a new active
    /// client this sender is passed down to the associated heartbeat
    /// timer.
    heartbeat_expirations_tx: mpsc::UnboundedSender<ClientId>,
    // start_training_expirations_tx: mpsc::UnvoundedSender<ClientId>,
    // done_training_expirations_tx: mpsc::UnboundedSender<ClientId>,
}

impl Clients {
    pub fn new(heartbeat_expirations_tx: mpsc::UnboundedSender<ClientId>) -> Self {
        Self {
            heartbeat_expirations_tx,
            waiting: HashMap::new(),
            selected: HashMap::new(),
            done: HashMap::new(),
            done_and_inactive: HashSet::new(),
            ignored: HashMap::new(),
        }
    }

    pub fn get_counters(&self) -> Counters {
        Counters {
            waiting: self.waiting.len() as u32,
            selected: self.selected.len() as u32,
            done: self.done.len() as u32,
            done_and_inactive: self.done_and_inactive.len() as u32,
            ignored: self.ignored.len() as u32,
        }
    }

    /// Create a new active client and its associated timer. It is the
    /// caller's responsability to spawn the timer.
    fn new_active_client(&self, id: ClientId) -> (ActiveClient, HeartBeatTimer) {
        let (heartbeat_reset_tx, heartbeat_reset_rx) = mpsc::channel::<Duration>(10);
        let heartbeat_timer = self.new_heartbeat_timer(id, heartbeat_reset_rx);
        let client = ActiveClient::new(heartbeat_reset_tx);
        (client, heartbeat_timer)
    }

    /// Create a new heartbeat timer.
    fn new_heartbeat_timer(
        &self,
        id: ClientId,
        resets_rx: mpsc::Receiver<Duration>,
    ) -> HeartBeatTimer {
        HeartBeatTimer::new(
            id,
            HEARTBEAT_TIMEOUT,
            self.heartbeat_expirations_tx.clone(),
            resets_rx,
        )
    }

    /// Return the state of the given client, whether it is active or
    /// not.
    pub fn get_state(&self, id: &ClientId) -> ClientState {
        let Self {
            waiting,
            selected,
            ignored,
            done,
            done_and_inactive,
            ..
        } = self;
        waiting
            .get(&id)
            .map(|_| ClientState::Waiting)
            .or_else(|| selected.get(&id).map(|_| ClientState::Selected))
            .or_else(|| ignored.get(&id).map(|_| ClientState::Ignored))
            .or_else(|| done.get(&id).map(|_| ClientState::Done))
            .or_else(|| {
                done_and_inactive
                    .get(&id)
                    .map(|_| ClientState::DoneAndInactive)
            })
            .unwrap_or(ClientState::Unknown)
    }

    /// Return whether the given client exists
    fn contains(&self, id: &ClientId) -> bool {
        self.get_state(id) != ClientState::Unknown
    }

    /// Return whether the given client exists and is active
    fn is_active(&self, id: &ClientId) -> bool {
        let state = self.get_state(id);
        state != ClientState::Unknown && state != ClientState::DoneAndInactive
    }

    /// Return whether the given client exists and is inactive
    fn is_inactive(&self, id: &ClientId) -> bool {
        self.done_and_inactive.contains(id)
    }

    /// Update the state of the given client. This is one very
    /// important but also quite tricky method to implement: getting
    /// it wrong would lead to inconsistencies with the state machine.
    pub fn set_state(
        &mut self,
        id: ClientId,
        new_state: ClientState,
    ) -> Result<Option<HeartBeatTimer>, InvalidClientStateTransition> {
        use ClientState::*;

        // First, check that the transition we're doing is valid.
        let current_state = self.get_state(&id);
        if !is_valid_transition(current_state, Selected) {
            return Err(InvalidClientStateTransition(current_state, new_state));
        }
        // otherwise we would have returned an error above
        assert!(self.contains(&id));

        if new_state == DoneAndInactive {
            // otherwise, we're doing a transition
            // DoneAndInactive->DoneAndInactive which is invalid.
            assert!(self.is_active(&id));
            // UNWRAP_SAFE: per assert! above
            self.remove_active(&id).unwrap();
            self.done_and_inactive.insert(id);
            return Ok(None);
        }

        let mut heartbeat_timer = None;

        let client = if self.is_inactive(&id) {
            self.remove_inactive(&id);
            let (new_client, new_heartbeat_timer) = self.new_active_client(id);
            *&mut heartbeat_timer = Some(new_heartbeat_timer);
            new_client
        } else {
            assert!(self.is_active(&id));
            // UNWRAP_SAFE: per assert! above
            self.remove_active(&id).unwrap()
        };

        assert!(new_state != DoneAndInactive);
        assert!(new_state != Unknown);

        match new_state {
            Waiting => self.waiting.insert(id, client),
            Selected => self.selected.insert(id, client),
            Done => self.done.insert(id, client),
            Ignored => self.ignored.insert(id, client),
            DoneAndInactive | Unknown => unreachable!(), // per assert! above
        };

        Ok(heartbeat_timer)
    }

    /// Return a mutable reference to the given active client
    fn get_active_mut(&mut self, id: &ClientId) -> Option<&mut ActiveClient> {
        let Self {
            waiting,
            selected,
            ignored,
            done,
            ..
        } = self;
        waiting
            .get_mut(&id)
            .or_else(move || selected.get_mut(&id))
            .or_else(move || ignored.get_mut(&id))
            .or_else(move || done.get_mut(&id))
    }

    /// Remove the given active client
    fn remove_active(&mut self, id: &ClientId) -> Option<ActiveClient> {
        let Self {
            waiting,
            selected,
            ignored,
            done,
            ..
        } = self;
        waiting
            .remove(id)
            .or_else(move || selected.remove(&id))
            .or_else(move || ignored.remove(&id))
            .or_else(move || done.remove(&id))
    }

    /// Remove the given inactive client
    fn remove_inactive(&mut self, id: &ClientId) -> Option<()> {
        self.done_and_inactive.remove(id).then_some(())
    }

    /// Reset the heartbeat timer of the given client
    pub fn reset_heartbeat(&mut self, id: &ClientId) -> Result<(), HeartBeatResetError> {
        self.get_active_mut(id)
            .ok_or(HeartBeatResetError::ClientNotFound)?
            .reset_heartbeat(HEARTBEAT_TIMEOUT)
    }

    pub fn add(&mut self, id: ClientId) -> HeartBeatTimer {
        let (client, heartbeat_timer) = self.new_active_client(id);
        self.waiting.insert(id, client);
        heartbeat_timer
    }

    pub fn remove(&mut self, id: &ClientId) -> Result<(), RemovedClientNotFound> {
        self.remove_active(id)
            .map(|_| ())
            .or_else(|| self.remove_inactive(id))
            .ok_or(RemovedClientNotFound(*id))
    }

    pub fn iter_waiting(&self) -> impl Iterator<Item = ClientId> + '_ {
        self.waiting.keys().cloned()
    }

    pub fn iter_selected(&self) -> impl Iterator<Item = ClientId> + '_ {
        let selected = self.selected.keys().cloned();
        let done = self.done.keys().cloned();
        let done_and_inactive = self.done_and_inactive.iter().cloned();
        selected.chain(done).chain(done_and_inactive)
    }

    pub fn reset(&mut self) {
        let selected = mem::replace(&mut self.selected, HashMap::new());
        let ignored = mem::replace(&mut self.ignored, HashMap::new());
        let done = mem::replace(&mut self.done, HashMap::new());

        self.waiting.extend(selected);
        self.waiting.extend(ignored);
        self.waiting.extend(done);
        self.done_and_inactive = HashSet::new();
    }
}

/// Error returned when reseting a heartbeat timer fails
#[derive(Debug, Display)]
pub enum HeartBeatResetError {
    /// The timer expired already
    #[display(fmt = "the heartbeat timer already expired")]
    Expired,

    /// Tried to reset the timer of a non-existent or inactive client
    #[display(fmt = "client not found")]
    ClientNotFound,

    /// Could not reset the timer due to too many pending resets
    #[display(fmt = "too many pending resets")]
    BackPressure,
}

impl Error for HeartBeatResetError {}

/// Return whether the transition from `current_state` to `new_state` is valid
fn is_valid_transition(current_state: ClientState, new_state: ClientState) -> bool {
    use ClientState::*;
    match (current_state, new_state) {
            | (Waiting, Selected)               // Waiting->Selected
            | (Selected, Done | Ignored)        // Selected->Done, Selected->Ignored
            | (Done, Ignored | DoneAndInactive) // Done->Ignored, Done->DoneAndInactive
            | (DoneAndInactive, Ignored)       // DoneAndInactive->Ignored
                => true,
            _ => false,
        }
}

/// Error returned when trying to update a client into an invalid state.
#[derive(Debug, Display)]
#[display(fmt = "invalid client state transition from {} to {}", _0, _1)]
pub struct InvalidClientStateTransition(ClientState, ClientState);

impl Error for InvalidClientStateTransition {}

/// Error returned when trying to remove a client that doesn't exist
#[derive(Debug, Display)]
#[display(fmt = "cannot remove client {}: not found", _0)]
pub struct RemovedClientNotFound(ClientId);

impl Error for RemovedClientNotFound {}
