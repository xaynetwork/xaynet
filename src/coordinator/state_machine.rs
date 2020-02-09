use derive_more::Display;
use std::collections::VecDeque;
use std::error::Error;

use crate::coordinator::client::ClientId;

#[derive(Eq, Debug, PartialEq, Default, Copy, Clone)]
pub struct Counters {
    /// Number of active clients waiting for being selected. These
    /// clients are in the [`ClientState::Waiting`] state.
    pub waiting: u32,
    /// Number of active client selected to take part to the current
    /// training round. These clients are in the
    /// [`ClientState::Selected`] state
    pub selected: u32,
    /// Number of client selected to take part to the current training
    /// round that already finishe training.
    pub done: u32,
    pub done_and_inactive: u32,
    pub ignored: u32,
}

impl Counters {
    pub fn new() -> Self {
        Default::default()
    }
}

/// The state machine.
pub struct StateMachine {
    counters: Counters,

    /// Whether all the round of training are done
    is_training_complete: bool,

    /// Coordinator configuration
    config: CoordinatorConfig,

    /// Current training round
    current_round: u32,

    /// Events emitted by the state machine
    events: VecDeque<Event>,
}

impl StateMachine {
    fn number_of_clients_to_select(&self) -> Option<u32> {
        if self.is_training_complete {
            return None;
        }

        let Counters {
            waiting,
            selected,
            done,
            done_and_inactive,
            ..
        } = self.counters;

        let total_participants = selected + done + done_and_inactive;
        if total_participants >= self.config.minimum_participants() {
            return None;
        }

        // We need to select more clients. But do we have enough
        // clients to perform the selection?
        let total_clients = total_participants + waiting;
        if total_clients < self.config.min_clients {
            return None;
        }

        let total_to_select =
            f64::ceil(self.config.participants_ratio * total_clients as f64) as i64 as u32;
        Some(total_to_select - total_participants)
    }

    fn maybe_start_selection(&mut self) {
        if let Some(count) = self.number_of_clients_to_select() {
            self.emit_event(Event::RunSelection(count))
        }
    }

    fn is_end_of_round(&self) -> bool {
        self.counters.selected == 0 && self.number_of_clients_to_select().is_none()
    }

    /// Emit an event
    fn emit_event(&mut self, event: Event) {
        self.events.push_back(event);
    }
}

// public methods
impl StateMachine {
    pub fn counters(&self) -> Counters {
        self.counters.clone()
    }
    pub fn new(config: CoordinatorConfig) -> Self {
        Self {
            config,
            counters: Counters::new(),
            is_training_complete: false,
            current_round: 0,
            events: VecDeque::new(),
        }
    }
    pub fn select(&mut self, mut candidates: impl Iterator<Item = (ClientId, ClientState)>) {
        if let Some(mut total_needed) = self.number_of_clients_to_select() {
            while total_needed > 0 {
                match candidates.next() {
                    Some((id, ClientState::Waiting)) => {
                        self.counters.selected += 1;
                        self.counters.waiting -= 1;
                        total_needed -= 1;
                        self.emit_event(Event::SetState(id, ClientState::Selected));
                    }
                    Some(_) => {}
                    None => {
                        break;
                    }
                }
            }
        }
        self.maybe_start_selection();
    }

    /// Handle a rendez-vous request for the given client.
    ///
    /// # Returns
    ///
    /// This method returns the response to send back to the client.
    pub fn rendez_vous(&mut self, id: ClientId, client_state: ClientState) -> RendezVousResponse {
        if self.is_training_complete {
            self.emit_event(Event::Remove(id));
            return RendezVousResponse::Reject;
        }
        let response = match client_state {
            ClientState::Unknown => {
                // Accept new clients and make them selectable
                self.counters.waiting += 1;
                self.emit_event(Event::SetState(id, ClientState::Waiting));
                RendezVousResponse::Accept
            }
            ClientState::Waiting => {
                // The client should not re-send a rendez-vous
                // request, but that can be the case if it got
                // re-started so let's accept the client again.
                RendezVousResponse::Accept
            }
            ClientState::Selected => {
                // A selected/training client should not send us
                // a rendez-vous request. Let's not rely on it
                // for that round but still accept it for the
                // next round. The idea is to mitigate attacks
                // when many clients connect to the coordinator
                // and drop out once selected, while not
                // penalizing honest clients that had a
                // connectivity issue.
                self.counters.selected -= 1;
                self.counters.ignored += 1;
                self.emit_event(Event::SetState(id, ClientState::Ignored));
                RendezVousResponse::Accept
            }
            ClientState::DoneAndInactive | ClientState::Done => {
                // A client that has finished training may send
                // us a rendez-vous request if it is
                // restarted. This is problematic because we
                // cannot put them back in the "Waiting"
                // state, otherwise they might be selected
                // again for the current training round, to
                // which they already participated. Therefore,
                // we accept these clients but mark them as
                // "Ignored", to exclude them from the
                // selection process.
                self.counters.ignored += 1;
                self.emit_event(Event::SetState(id, ClientState::Ignored));
                RendezVousResponse::Accept
            }
            ClientState::Ignored => RendezVousResponse::Accept,
        };
        self.maybe_start_selection();
        response
    }

    /// Handle a heartbeat timeout for the given client.
    pub fn hearbeat_timeout(&mut self, id: ClientId, client_state: ClientState) {
        self.emit_event(Event::Remove(id));
        match client_state {
            ClientState::Selected => self.counters.selected -= 1,
            ClientState::Waiting => self.counters.waiting -= 1,
            ClientState::Unknown => {
                panic!("Unknown client {} does not have a heartbeat", id);
            }
            ClientState::DoneAndInactive => {
                panic!("Done and inactive client {} does not have a heartbeat", id);
            }
            ClientState::Done => {
                self.emit_event(Event::SetState(id, ClientState::DoneAndInactive));
                self.counters.done_and_inactive += 1;
            }
            ClientState::Ignored => {
                self.counters.ignored -= 1;
            }
        }
        self.maybe_start_selection();
    }

    /// Handle a heartbeat for the given client.
    ///
    /// # Returns
    ///
    /// This method returns the response to send back to the client.
    pub fn heartbeat(&mut self, id: ClientId, client_state: ClientState) -> HeartBeatResponse {
        if self.is_training_complete {
            self.emit_event(Event::ResetHeartBeat(id));
            return HeartBeatResponse::Finish;
        }
        match client_state {
            // Reject any client we don't know about. They must first
            // send a rendez-vous request to be recognized by the
            // coordinator.
            ClientState::Unknown => HeartBeatResponse::Reject,

            // The client may have come back to life. But once a
            // client has become inactive, it has to send a new
            // rendez-vous request and be accepted by the coordinator,
            // so we reject this heartbeat.
            ClientState::DoneAndInactive => HeartBeatResponse::Reject,

            // Client that are waiting or done should stand by
            ClientState::Ignored | ClientState::Waiting | ClientState::Done => {
                self.emit_event(Event::ResetHeartBeat(id));
                HeartBeatResponse::StandBy
            }

            // If the client has been selected, notify them.
            ClientState::Selected => {
                self.emit_event(Event::ResetHeartBeat(id));
                HeartBeatResponse::Round(self.current_round)
            }
        }
    }

    /// Handle a start training request for the given client.
    ///
    /// # Returns
    ///
    /// This method returns the response to send back to the client.
    pub fn start_training(&mut self, client_state: ClientState) -> StartTrainingResponse {
        // FIXME: Can this be a vector for DoS attacks? In the "start
        // training" response we send the latest aggregated model
        // which can be big. If many selected clients send lots of
        // "start training" request, we may end up serving gigabytes
        // of data. One way to mitigate this could be to keep track of
        // the clients that already sent such a request. These clients
        // would be in the "Training" state.
        if client_state == ClientState::Selected && !self.is_training_complete {
            StartTrainingResponse::Accept
        } else {
            StartTrainingResponse::Reject
        }
    }

    /// Handle an end training request for the given client.
    ///
    /// # Returns
    ///
    /// This method returns the response to send back to the client.
    pub fn end_training(&mut self, client_state: ClientState) -> EndTrainingResponse {
        if self.is_training_complete {
            return EndTrainingResponse::Reject;
        }

        if client_state == ClientState::Selected {
            self.counters.selected -= 1;
            self.counters.done += 1;
            if self.is_end_of_round() {
                self.current_round += 1;
                if self.current_round == self.config.rounds {
                    self.is_training_complete = true;
                } else {
                    self.emit_event(Event::ResetAll);
                    self.counters.waiting += self.counters.done;
                    self.counters.waiting += self.counters.ignored;
                    self.counters.done_and_inactive = 0;
                    self.counters.ignored = 0;
                }
            }
            self.maybe_start_selection();
            EndTrainingResponse::Accept
        } else {
            EndTrainingResponse::Reject
        }
    }

    /// Retrieve the next event
    pub fn next_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }
}

pub struct CoordinatorConfig {
    rounds: u32,
    participants_ratio: f64,
    min_clients: u32,
    epoch: u32,
}

impl CoordinatorConfig {
    fn minimum_participants(&self) -> u32 {
        (self.participants_ratio * self.min_clients as f64) as i64 as u32
    }
}

/// Response to a heartbeat
pub enum HeartBeatResponse {
    /// The client should stand by in its current state
    StandBy,

    /// The coordinator has finished, and the client should disconnect
    Finish,

    /// The client has been selected for the given round and should
    /// start or continue training
    Round(u32),

    /// The client has not been accepted by the coordinator yet and
    /// should not send heartbeats
    Reject,
}

/// Response to a "start training" request.
pub enum StartTrainingResponse {
    Reject,
    Accept,
}

/// Response to a "end training" request.
pub enum EndTrainingResponse {
    Accept,
    Reject,
}
//     pub global_weights: f64,
//     pub epochs: u32,
// }

/// Response to a rendez-vous request
pub enum RendezVousResponse {
    /// The coordinator accepts the client
    Accept,

    /// The coordinator rejects the client
    Reject,
}

/// Represent the state of a client, as seen by the state machine
#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone, Display)]
pub enum ClientState {
    /// The client has not sent a rendez-vous request yet
    Unknown,
    /// The client has sent a rendez-vous request but has not been
    /// selected for a training round
    Waiting,
    /// The client has been selected for the current training round but
    /// hasn't started training yet
    Selected,
    // /// The client has been selected for the current training round and
    // /// has started training
    // Training,
    /// The client has been selected for the current training round and
    /// has finished training
    Done,
    /// The client has been selected for the current training round and
    /// has finished training but disconnected
    DoneAndInactive,
    /// The client is alive but excluded from the selection
    Ignored,
}

/// Events emitted by the state machine
pub enum Event {
    /// Accept the given client. This client becomes selectable, _ie_
    /// has state [`ClientState::Waiting`].
    Accept(ClientId),

    /// Remove a client. This client becomes unknown [`ClientState::Unknown`].
    Remove(ClientId),

    /// Update the given client's state.
    SetState(ClientId, ClientState),

    /// Reset all the active clients to their default state:
    /// [`ClientState::Waiting`], and remove the inactive clients.
    ResetAll,

    /// Reset the hearbeat timer for the given client
    ResetHeartBeat(ClientId),

    /// Start the aggregation process
    RunAggregation,

    /// Start the selection process
    RunSelection(u32),
}

#[derive(Debug, Display)]
pub struct InvalidState;
impl Error for InvalidState {}
