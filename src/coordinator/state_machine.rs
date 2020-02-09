use derive_more::Display;
use std::collections::VecDeque;
use std::error::Error;

use crate::coordinator::client::ClientId;

/// Current state of the state machine
#[derive(Eq, PartialEq, Hash, Debug)]
enum State {
    StandBy,
    Round,
    Finished,
}

/// The state machine.
pub struct StateMachine {
    /// Number of active client selected to take part to the current
    /// training round. These clients are in the
    /// [`ClientState::Selected`] state
    selected_counter: u32,

    /// Number of client selected to take part to the current training
    /// round that already finishe training. These clients are either
    /// in the [`ClientState::Done`] if they are still active or
    /// [`ClientState::DoneAndInactive`] if they are not active anymore.
    done_counter: u32,

    /// Number of active clients waiting for being selected. These
    /// clients are in the [`ClientState::Waiting`] state.
    waiting_counter: u32,

    /// Current state of the coordinator
    state: State,

    /// Coordinator configuration
    config: CoordinatorConfig,

    /// Current training round
    current_round: u32,

    /// Events emitted by the state machine
    events: VecDeque<Event>,
}

impl StateMachine {
    /// Return the number of _selected_ and _selectable_ clients.
    fn total_clients(&self) -> u32 {
        self.waiting_counter + self.selected_counter + self.done_counter
    }

    /// Return the total number of clients that are taking part to the
    /// current round.
    fn total_participants(&self) -> u32 {
        self.selected_counter + self.done_counter
    }

    /// Return the ratio of clients that participate to the current round.
    fn participants_ratio(&self) -> f64 {
        if self.total_clients() == 0 {
            0f64
        } else {
            f64::from(self.total_participants()) / f64::from(self.total_clients())
        }
    }

    /// Return `true` if there are enough participants to (re)start
    /// a training round.
    fn has_enough_participants(&self) -> bool {
        self.participants_ratio() >= self.config.participants_ratio
    }

    /// Return `true` is there are enough clients to (re)start a
    /// training round
    fn has_enough_clients(&self) -> bool {
        self.total_clients() >= self.config.min_clients
    }

    fn should_start_selection(&self) -> bool {
        self.state == State::StandBy && self.has_enough_clients() && !self.has_enough_participants()
    }

    fn should_continue_round(&self) -> bool {
        self.state == State::Round
            && (!self.has_enough_clients() || !self.has_enough_participants())
    }

    /// Check how many clients we have and how many of them are
    /// currently selected. If necessary, update the state and/or
    /// start a new selection round. The method should be called every
    /// time a client is accepted by the coordinator or disconnects.
    // FIXME: we should introduce hysteresis to prevent the state
    // machine from constantly switching between StandBy and Round and
    // performing selections when we are around the threshold for
    // starting a round.
    //
    // FIXME2: if more clients connect, the ratio of participants goes
    // down, so we'll have to select more participants. Here again we
    // may need to introduce hysteresis. In our former Python
    // implementation we "solved" this issue by preventing new clients
    // to connect (rejecting Rendez-Vous while in a round), but it has
    // its own downside: when we lose participants, we may not be able
    // to re-run a selection right away because we may not have enough
    // clients.
    fn on_counter_update(&mut self) {
        match &self.state {
            State::StandBy => {
                if self.should_start_selection() {
                    self.emit_event(Event::RunSelection(self.number_of_clients_to_select()));
                }
            }
            State::Round => {
                if !self.should_continue_round() {
                    self.state = State::StandBy;
                }
                if self.should_start_selection() {
                    self.emit_event(Event::RunSelection(self.number_of_clients_to_select()));
                }
            }
            State::Finished => {}
        }
    }
    fn number_of_clients_to_select(&self) -> u32 {
        // FIXME: check rules for casting between floats,
        // signed/unsigned integers, etc.  This i64 as u32 is probably
        // not right.
        let total_to_select =
            f64::ceil(self.config.participants_ratio * self.total_clients() as f64) as i64 as u32;
        assert!(total_to_select >= self.selected_counter);
        total_to_select - self.selected_counter
    }
    /// Increment the counter for waiting clients and update the
    /// state machine accordingly
    fn incr_waiting(&mut self) {
        if self.waiting_counter > 0 {
            self.waiting_counter += 1;
        }
        self.on_counter_update()
    }

    /// Decrement the counter for waiting clients and update the
    /// state machine accordingly
    fn decr_waiting(&mut self) {
        if self.waiting_counter > 0 {
            self.waiting_counter -= 1;
        } else {
            panic!("tried to decrement null waiting_counter counter");
        }
        self.on_counter_update()
    }

    /// Decrement the counter for selected clients and update the
    /// state machine accordingly
    fn decr_selected(&mut self) {
        if self.selected_counter > 0 {
            self.selected_counter -= 1;
        } else {
            panic!("tried to decrement null selected_counter counter");
        }
        self.on_counter_update()
    }

    /// Emit an event
    fn emit_event(&mut self, event: Event) {
        self.events.push_back(event);
    }
}

// public methods
impl StateMachine {
    pub fn select(&mut self, mut candidates: impl Iterator<Item = (ClientId, ClientState)>) {
        match self.state {
            State::Finished | State::Round => {}
            State::StandBy => {
                let mut total_needed = self.number_of_clients_to_select();
                while total_needed > 0 {
                    match candidates.next() {
                        Some((id, ClientState::Waiting)) => {
                            self.selected_counter += 1;
                            self.waiting_counter -= 1;
                            total_needed -= 1;
                            self.emit_event(Event::SetState(id, ClientState::Selected));
                        }
                        Some(_) => {}
                        None => {
                            break;
                        }
                    }
                }
                self.on_counter_update();
            }
        }
    }

    /// Handle a rendez-vous request for the given client.
    ///
    /// # Returns
    ///
    /// This method returns the response to send back to the client.
    pub fn rendez_vous(&mut self, id: ClientId, client_state: ClientState) -> RendezVousResponse {
        match self.state {
            State::Round | State::StandBy => {
                match client_state {
                    ClientState::Unknown => {
                        // Accept new clients and make them selectable
                        self.incr_waiting();
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
                        self.decr_selected();
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
                        self.emit_event(Event::SetState(id, ClientState::Ignored));
                        RendezVousResponse::Accept
                    }
                    ClientState::Ignored => RendezVousResponse::Accept,
                }
            }
            State::Finished => {
                self.emit_event(Event::Remove(id));
                RendezVousResponse::Reject
            }
        }
    }

    /// Handle a heartbeat timeout for the given client.
    pub fn hearbeat_timeout(&mut self, id: ClientId, client_state: ClientState) {
        self.emit_event(Event::Remove(id));
        match client_state {
            ClientState::Selected => self.decr_selected(),
            ClientState::Waiting => self.decr_waiting(),
            ClientState::Unknown => {
                panic!("Unknown client {} does not have a heartbeat", id);
            }
            ClientState::DoneAndInactive => {
                panic!("Done and inactive client {} does not have a heartbeat", id);
            }
            _ => {}
        }
    }

    /// Handle a heartbeat for the given client.
    ///
    /// # Returns
    ///
    /// This method returns the response to send back to the client.
    pub fn heartbeat(&mut self, id: ClientId, client_state: ClientState) -> HeartBeatResponse {
        match (&self.state, client_state) {
            // Reject any client we don't know about. They must first
            // send a rendez-vous request to be recognized by the
            // coordinator.
            (_, ClientState::Unknown) => HeartBeatResponse::Reject,

            // The client may have come back to life. But once a
            // client has become inactive, it has to send a new
            // rendez-vous request and be accepted by the coordinator,
            // so we reject this heartbeat.
            (_, ClientState::DoneAndInactive) => HeartBeatResponse::Reject,

            (State::Finished, _) => {
                self.emit_event(Event::ResetHeartBeat(id));
                HeartBeatResponse::Finish
            }

            // Client that are waiting or done should stand by
            (
                State::Round | State::StandBy,
                ClientState::Ignored | ClientState::Waiting | ClientState::Done,
            ) => {
                self.emit_event(Event::ResetHeartBeat(id));
                HeartBeatResponse::StandBy
            }

            // If the client has been selected, notify them.
            (State::StandBy | State::Round, ClientState::Selected) => {
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
        match (&self.state, client_state) {
            // FIXME: Can this be a vector for DoS attacks? In the
            // "start training" response we send the latest aggregated
            // model which can be big. If many selected clients send
            // lots of "start training" request, we may end up serving
            // gigabytes of data. One way to mitigate this could be to
            // keep track of the clients that already sent such a
            // request. These clients would be in the "Training"
            // state.
            (State::StandBy | State::Round, ClientState::Selected) => StartTrainingResponse::Accept,
            _ => StartTrainingResponse::Reject,
        }
    }

    /// Handle an end training request for the given client.
    ///
    /// # Returns
    ///
    /// This method returns the response to send back to the client.
    pub fn end_training(&mut self, client_state: ClientState) -> EndTrainingResponse {
        match (&self.state, client_state) {
            (State::StandBy | State::Round, ClientState::Selected) => EndTrainingResponse::Accept,
            _ => EndTrainingResponse::Reject,
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
