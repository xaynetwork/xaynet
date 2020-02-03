use super::heartbeat::*;
use super::state_machine::*;
use derive_more::Display;
use std::{collections::HashMap, error::Error, time::Duration};
use tokio::sync::mpsc;
use uuid::Uuid;

const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(10);
const HEARTBEAT_TIME: Duration = Duration::from_secs(5);

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

/// A client, as seen by the coordinator.
pub enum Client {
    Active(ActiveClient),
    Inactive,
}

impl Client {
    /// Return whether this client is active, _ie_ is sending regular
    /// heartbeat messages.
    pub fn is_active(&self) -> bool {
        if let Self::Active(_) = self {
            true
        } else {
            false
        }
    }

    /// Reset the client's heartbeat timer.
    pub fn reset_heartbeat(&mut self, timeout: Duration) -> Result<(), HeartBeatResetError> {
        use Client::*;
        match self {
            Active(active_client) => {
                active_client.heartbeat_reset.try_send(timeout).map_err(|err| {
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
            Inactive => Err(HeartBeatResetError::InactiveClient),
        }
    }

    // pub fn set_state(&mut self, state: ClientState) {
    //     match self {
    //         Self::Waiting(c) if state == ClientState::Selected => {
    //             let

    //         }
    //         Self::Selected(c) => match state {
    //             ClientState::Done => {}
    //             ClientState::Ignored => {}
    //             _ => {}
    //         },
    //         Self::DoneAndInactive if state == ClientState::Ignored => {}
    //         _ => {}
    //     }
    // }
}

/// A store for all the clients the coordinator is tracking.
pub struct Clients {
    /// Active clients that are not selected for the current training
    /// round but could be selected at some point. It corresponds to
    /// clients in state [`ClientState::Waiting`],
    waiting: HashMap<ClientId, Client>,

    /// Active clients that are selected for the current training
    /// round, but haven't finish training. It corresponds to clients
    /// in state [`ClientState::Selected`], [`ClientState::Training`]
    selected: HashMap<ClientId, Client>,

    /// Active clients that cannot be selected for the current
    /// round. It corresponds to clients in state
    /// [`ClientState::Ignored`].
    ignored: HashMap<ClientId, Client>,

    /// Active clients that took part to the current training round
    /// and finished training. I corresponds to clients in state
    /// [`ClientState::Done`].
    done: HashMap<ClientId, Client>,

    /// Clients that were selected for the current training round and
    /// that finished training their model, but that are not active
    /// anymore. It corresponds to clients in state
    /// [`ClientState::DoneAndInactive`]
    done_and_inactive: HashMap<ClientId, Client>,

    heartbeat_expirations_tx: mpsc::UnboundedSender<ClientId>,
    // start_training_expirations_tx: mpsc::UnvoundedSender<ClientId>,
    // done_training_expirations_tx: mpsc::UnboundedSender<ClientId>,
}

impl Clients {
    /// Create a new active client and its associated timer. It is the
    /// caller's responsability to spawn the timer.
    fn new_client(&self, id: ClientId) -> (Client, HeartBeatTimer) {
        let (heartbeat_reset_tx, heartbeat_reset_rx) = mpsc::channel::<Duration>(10);
        let heartbeat_timer = HeartBeatTimer::new(
            id,
            HEARTBEAT_TIMEOUT,
            self.heartbeat_expirations_tx.clone(),
            heartbeat_reset_rx,
        );
        let client = Client::Active(ActiveClient {
            heartbeat_reset: heartbeat_reset_tx,
        });
        (client, heartbeat_timer)
    }

    /// Return the state of the given client
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

    /// Update the state of the given client. This is one very
    /// important but also quite tricky method to implement: getting
    /// it wrong would lead to inconsistencies with the state machine.
    pub fn set_state(
        &mut self,
        id: ClientId,
        new_state: ClientState,
    ) -> Result<Option<HeartBeatTimer>, InvalidClientStateError> {
        use ClientState::*;

        // First, check that the transition we're doing is valid.
        let current_state = self.get_state(&id);
        if !self.is_valid_transition(current_state, Selected) {
            return Err(InvalidClientStateError(current_state, new_state));
        }
        // There is no valid transition from Unknown. So if
        // current_state was Unknown, we would have returned an error
        // above.
        assert!(current_state != Unknown);

        // UNWRAP_SAFE: current_state != Unknown, so we have that
        // client for sure.
        let mut client = self.remove(&id).unwrap();

        // If the current client is inactive and we're transitioning
        // to an active client, we need to create a new client
        // instance along with its heartbeat timer. Likewise, if the
        // current client is active and we're transitioning to an
        // inactive client, we create a new client instance.
        let mut heartbeat_timer = None;
        if new_state != DoneAndInactive && !client.is_active() {
            let (new_client, new_heartbeat_timer) = self.new_client(id);
            *&mut client = new_client;
            *&mut heartbeat_timer = Some(new_heartbeat_timer);
        } else if new_state == DoneAndInactive && client.is_active() {
            *&mut client = Client::Inactive;
        }

        // Set the new state by inserting the client in the right
        // hashmap.
        //
        // FIXME: I realize here that it would be better for the
        // hashmap to store the inner client instead of the `Client`
        // wrapper type, because we could ensure at compile time that
        // we're inserting the right type of client in each map.
        //
        // But then we cannot have convenient methods such as
        // `Clients::get()` and `Clients::get_mut()` which return just
        // a `Client`. Instead we would need `Client::get_active()`
        // and `Client::get_inactive()` variants.
        match new_state {
            Waiting => self.waiting.insert(id, client),
            Selected => self.selected.insert(id, client),
            Done => self.done.insert(id, client),
            DoneAndInactive => self.done_and_inactive.insert(id, client),
            Ignored => self.ignored.insert(id, client),
            Unknown => return Err(InvalidClientStateError(current_state, new_state)),
        };

        // In case we had to create a new client and its associated
        // timer, return the timer so that the caller can spawn it.
        Ok(heartbeat_timer)
    }

    /// Return whether the transition from `current_state` to `new_state` is valid
    fn is_valid_transition(&self, current_state: ClientState, new_state: ClientState) -> bool {
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

    /// Return a reference to the given client
    fn get(&self, id: &ClientId) -> Option<&Client> {
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
            .or_else(|| selected.get(&id))
            .or_else(|| ignored.get(&id))
            .or_else(|| done.get(&id))
            .or_else(|| done_and_inactive.get(&id))
    }

    /// Return a mutable reference to the given client
    fn get_mut(&mut self, id: &ClientId) -> Option<&mut Client> {
        let Self {
            waiting,
            selected,
            ignored,
            done,
            done_and_inactive,
            ..
        } = self;
        waiting
            .get_mut(&id)
            .or_else(move || selected.get_mut(&id))
            .or_else(move || ignored.get_mut(&id))
            .or_else(move || done.get_mut(&id))
            .or_else(move || done_and_inactive.get_mut(&id))
    }

    /// Reset the heartbeat timer of the given client
    fn reset_heartbeat(
        &mut self,
        id: &ClientId,
        timeout: Duration,
    ) -> Result<(), HeartBeatResetError> {
        self.get_mut(id)
            .ok_or(HeartBeatResetError::ClientNotFound)?
            .reset_heartbeat(timeout)
    }

    fn remove(&mut self, id: &ClientId) -> Option<Client> {
        let Self {
            waiting,
            selected,
            ignored,
            done,
            done_and_inactive,
            ..
        } = self;
        waiting
            .remove(id)
            .or_else(move || selected.remove(&id))
            .or_else(move || ignored.remove(&id))
            .or_else(move || done.remove(&id))
            .or_else(move || done_and_inactive.remove(&id))
    }
}

/// Error returned when reseting a heartbeat timer fails
#[derive(Debug, Display)]
pub enum HeartBeatResetError {
    /// The timer expired already
    #[display(fmt = "the heartbeat timer already expired")]
    Expired,

    /// This client does not have a heartbeat timer running because
    /// it's inactive
    #[display(fmt = "client is inactive")]
    InactiveClient,

    /// Tried to reset the timer of a non-existent client
    #[display(fmt = "client not found")]
    ClientNotFound,

    /// Could not reset the timer due to too many pending resets
    #[display(fmt = "too many pending resets")]
    BackPressure,
}

impl Error for HeartBeatResetError {}

/// Error returned when trying to update a client into an invalid state.
#[derive(Debug, Display)]
#[display(fmt = "invalid client state transition from {} to {}", _0, _1)]
pub struct InvalidClientStateError(ClientState, ClientState);

impl Error for InvalidClientStateError {}
