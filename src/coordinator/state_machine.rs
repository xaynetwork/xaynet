#![allow(dead_code)]

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
    clients_selected: u32,

    /// Number of client selected to take part to the current training
    /// round that already finishe training. These clients are either
    /// in the [`ClientState::Done`] if they are still active or
    /// [`ClientState::DoneAndInactive`] if they are not active anymore.
    clients_done: u32,

    /// Number of active clients waiting for being selected. These
    /// clients are in the [`ClientState::Waiting`] state.
    clients_waiting: u32,

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
        self.clients_waiting + self.clients_selected + self.clients_done
    }

    /// Return the total number of clients that are taking part to the
    /// current round.
    fn total_participants(&self) -> u32 {
        self.clients_selected + self.clients_done
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
    /// the current round.
    fn has_enough_participants(&self) -> bool {
        self.participants_ratio() >= self.config.participants_ratio
    }

    /// Emit a transition event if necessary
    fn maybe_transition(&mut self) {
        let current_state = &self.state;
        let has_enough_participants = self.has_enough_participants();
        let did_enough_rounds = self.current_round > self.config.rounds;
        // self.transition_event =
        Some(
            match (current_state, did_enough_rounds, has_enough_participants) {
                (State::StandBy | State::Round, true, _) => State::Finished,
                (State::StandBy, false, true) => State::Round,
                (State::Round, false, false) => State::StandBy,
                _ => return,
            },
        );
    }

    pub fn handle_rendez_vous(
        &mut self,
        id: ClientId,
        client_state: ClientState,
    ) -> RendezVousResponse {
        match self.state {
            State::Round | State::StandBy => {
                match client_state {
                    ClientState::Unknown => {
                        // Accept new clients and make them selectable
                        self.incr_waiting();
                        self.events
                            .push_back(Event::SetState(id, ClientState::Waiting));
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
                        self.events
                            .push_back(Event::SetState(id, ClientState::Ignored));
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
                        self.events
                            .push_back(Event::SetState(id, ClientState::Ignored));
                        RendezVousResponse::Accept
                    }
                    ClientState::Ignored => RendezVousResponse::Accept,
                }
            }
            State::Finished => {
                self.events.push_back(Event::Remove(id));
                RendezVousResponse::Reject
            }
        }
    }

    /// Handle a heartbeat timeout.
    pub fn handle_hearbeat_timeout(
        &mut self,
        id: ClientId,
        client_state: ClientState,
    ) -> Result<(), InvalidStateError> {
        self.events.push_back(Event::Remove(id));
        match client_state {
            ClientState::Selected => self.decr_selected(),
            ClientState::Waiting => self.decr_waiting(),
            ClientState::Unknown => {
                return Err(InvalidStateError(format!(
                    "Unknown client {} does not have a heartbeat",
                    id
                )))
            }
            ClientState::DoneAndInactive => {
                return Err(InvalidStateError(format!(
                    "Done and inactive client {} does not have a heartbeat",
                    id
                )))
            }
            _ => {}
        }
        Ok(())
    }

    pub fn handle_heartbeat(
        &mut self,
        id: ClientId,
        client_state: ClientState,
    ) -> HeartBeatResponse {
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
                self.events.push_back(Event::ResetHeartBeat(id));
                HeartBeatResponse::Finish
            }

            // Client that are waiting or done should stand by
            (
                State::Round | State::StandBy,
                ClientState::Ignored | ClientState::Waiting | ClientState::Done,
            ) => {
                self.events.push_back(Event::ResetHeartBeat(id));
                HeartBeatResponse::StandBy
            }

            // If the client has been selected, notify them.
            (State::StandBy | State::Round, ClientState::Selected) => {
                self.events.push_back(Event::ResetHeartBeat(id));
                HeartBeatResponse::Round(self.current_round)
            }
        }
    }

    fn incr_selected(&mut self) {
        if self.clients_selected > 0 {
            self.clients_selected += 1;
        }
        self.maybe_transition();
    }

    fn incr_waiting(&mut self) {
        if self.clients_waiting > 0 {
            self.clients_waiting += 1;
        }
        self.maybe_transition();
    }

    fn incr_done(&mut self) {
        if self.clients_done > 0 {
            self.clients_done += 1;
        }
        self.maybe_transition();
    }

    fn decr_selected(&mut self) {
        if self.clients_selected > 0 {
            self.clients_selected -= 1;
        } else {
            panic!("tried to decrement null clients_selected counter");
        }
        self.maybe_transition();
    }

    fn decr_waiting(&mut self) {
        if self.clients_waiting > 0 {
            self.clients_waiting -= 1;
        } else {
            panic!("tried to decrement null clients_waiting counter");
        }
        self.maybe_transition();
    }

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

pub enum HeartBeatResponse {
    StandBy,
    Finish,
    Round(u32),
    Reject,
}

pub struct StartTrainingResponse {
    pub global_weights: f64,
    pub epochs: u32,
}

pub enum RendezVousResponse {
    Accept,
    Reject,
}

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

/// A trait that helps implementing the coordinator protocol by
/// defining the methods that should be implemented in order to handle
/// all the events the state machine can emit.
pub trait StateMachineEventHandler {
    /// Handle a [`Event::Accept`] event
    fn accept_client(&mut self, id: ClientId);

    /// Handle a [`Event::Remove`] event
    fn remove_client(&mut self, id: ClientId);

    /// Handle a [`Event::ResetAll`] event
    fn reset_all_clients(&mut self);

    /// Handle a [`Event::SetState`] event
    fn set_client_state(&mut self, id: ClientId, client_state: ClientState);

    /// Handle a [`Event::ResetHeartBeat`] event
    fn reset_heartbeat(&mut self, id: ClientId);

    /// Handle a [`Event::StartAggregation`] event
    fn start_aggregation(&mut self) {
        unimplemented!()
    }

    /// Handle a [`Event::StartSelection`] event
    fn start_selection(&mut self) {
        unimplemented!()
    }

    /// Dispatch an [`Event`] to the appropriate handler
    fn dispatch_event(&mut self, event: Event) {
        match event {
            Event::Accept(id) => self.accept_client(id),
            Event::Remove(id) => self.remove_client(id),
            Event::SetState(id, client_state) => self.set_client_state(id, client_state),
            Event::ResetAll => self.reset_all_clients(),
            Event::ResetHeartBeat(id) => self.reset_heartbeat(id),
            Event::StartAggregation => self.start_aggregation(),
            Event::StartSelection => self.start_selection(),
        }
    }
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
    StartAggregation,

    /// Start the selection process
    StartSelection,
}

#[derive(Debug, Display)]
#[display(fmt = "Invalid state machine state: {}", _0)]
pub struct InvalidStateError(String);
impl Error for InvalidStateError {}
