// Notes about a potential flaw in our current implementation
//
// 1. client restarts just becore sending an heartbeat and sends rdv request
// 2. state machine emit event to accept but ignore the client
// 3. we reset the client's heartbeat but the client's heartbeat expires just before (this kind of race is possible)
// 4. we remove the client
// 5. subsequent heartbeats are rejected.
// 6. the client restarts
// 7. we accept the client and make it selectable => this can be a problem if the client already took part to the round.
//
// Steps 5. and 7. are problematic, but how much? The race at step
// 3. is very unlikely, but we may still run into it.

use crate::coordinator::{Aggregator, Selector};

use super::client::*;
use super::handle::CoordinatorHandle;
use super::request::*;
use super::state_machine::{
    ClientState, CoordinatorConfig, Event, StartTrainingResponse as RawStartTrainingResponse,
    StateMachine,
};

use tokio::sync::mpsc;

use futures::{ready, stream::Stream};

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(10);
const HEARTBEAT_TIME: Duration = Duration::from_secs(5);

pub struct CoordinatorService<A, S, T>
where
    A: Aggregator<T>,
    T: Clone,
    S: Selector,
{
    requests_rx: mpsc::Receiver<Request<T>>,
    heartbeat_expirations_rx: mpsc::UnboundedReceiver<ClientId>,
    // start_training_expirations_rx: mpsc::UnboundedReceiver<ClientId>,
    // done_training_expirations_rx: mpsc::UnboundedReceiver<ClientId>,
    state_machine: StateMachine,
    clients: Clients,
    aggregator: A,
    selector: S,

    pending_selection: Vec<ClientId>,
    global_weights: T,
}

impl<A, S, T> CoordinatorService<A, S, T>
where
    A: Aggregator<T>,
    T: Clone,
    S: Selector,
{
    pub fn new(
        aggregator: A,
        selector: S,
        global_weights: T,
        config: CoordinatorConfig,
    ) -> (Self, CoordinatorHandle<T>) {
        let (requests_tx, requests_rx) = mpsc::channel(2048);
        let (heartbeat_expirations_tx, heartbeat_expirations_rx) = mpsc::unbounded_channel();

        let coordinator = Self {
            aggregator,
            selector,
            global_weights,
            heartbeat_expirations_rx,
            requests_rx,
            clients: Clients::new(heartbeat_expirations_tx),
            state_machine: StateMachine::new(config),
            pending_selection: Vec::new(),
        };
        let handle = CoordinatorHandle::new(requests_tx);
        (coordinator, handle)
    }
    /// Handle the pending state machine events.
    fn handle_state_machine_events(&mut self) {
        while let Some(event) = self.state_machine.next_event() {
            self.dispatch_event(event);
            self.sanity_checks();
        }
    }

    /// Handle the incoming requests.
    fn poll_requests(&mut self, cx: &mut Context) -> Poll<Option<()>> {
        loop {
            match ready!(Pin::new(&mut self.requests_rx).poll_next(cx)) {
                Some(request) => {
                    self.handle_request(request);
                    self.handle_state_machine_events();
                }
                None => return Poll::Ready(None),
            }
        }
    }

    /// Handle a request
    fn handle_request(&mut self, request: Request<T>) {
        match request {
            Request::RendezVous((opt_id, sender)) => {
                // There can be no ID provided, if this is the first
                // time the client sends a rendez-vous request. In
                // that case we generate one.
                let id = opt_id.unwrap_or_else(|| ClientId::new());
                // Find the client status by ID, defaulting to
                // Unknown.
                let status = self.clients.get_state(&id);
                let response = self.state_machine.rendez_vous(id, status);
                sender.send(response.into());
            }
            Request::HeartBeat((id, sender)) => {
                let response = self
                    .state_machine
                    .heartbeat(id, self.clients.get_state(&id));
                sender.send(response);
            }
            Request::StartTraining((id, sender)) => {
                let response = match self
                    .state_machine
                    .start_training(self.clients.get_state(&id))
                {
                    RawStartTrainingResponse::Accept => {
                        Ok(StartTrainingPayload::new(self.global_weights.clone()))
                    }
                    RawStartTrainingResponse::Reject => Err(()),
                };
                sender.send(response);
            }
            Request::EndTraining((id, sender)) => {
                let response = self.state_machine.end_training(self.clients.get_state(&id));
                sender.send(response);
            }
        }
    }

    fn apply_pending_selection(&mut self) {
        let Self {
            ref mut pending_selection,
            ref mut state_machine,
            ref mut clients,
            ..
        } = self;
        if !pending_selection.is_empty() {
            let chunk = pending_selection
                .drain(0..100)
                .map(|id| (id, clients.get_state(&id)));
            state_machine.select(chunk);
        }

        self.handle_state_machine_events();
    }

    fn sanity_checks(&self) {
        assert_eq!(self.clients.get_counters(), self.state_machine.counters());
    }
}

impl<A, S, T> Future for CoordinatorService<A, S, T>
where
    // FIXME: I guess it's OK to require Unpin for the aggregator and
    // the selector ? Unless it is not ?
    A: Aggregator<T> + Unpin,
    S: Selector + Unpin,
    T: Clone + Unpin,
{
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        trace!("polling Connection");
        let pin = self.get_mut();

        loop {
            match pin.poll_requests(cx) {
                Poll::Ready(Some(())) => {}
                Poll::Ready(None) => return Poll::Ready(()),
                Poll::Pending => break,
            }
        }

        pin.apply_pending_selection();

        Poll::Pending
    }
}

impl<A, S, T> CoordinatorService<A, S, T>
where
    A: Aggregator<T>,
    T: Clone,
    S: Selector,
{
    /// Handle a [`Event::Accept`] event
    fn accept_client(&mut self, id: ClientId) {
        let heartbeat_timer = self.clients.add(id);
        tokio::spawn(heartbeat_timer);
    }

    /// Handle a [`Event::Remove`] event
    fn remove_client(&mut self, id: ClientId) {
        // If our implementation is correct, this should never return
        // an error. If it does, our state is invalid, so it is OK to
        // panic.
        self.clients.remove(&id).expect("failed to remove client");
    }

    /// Handle a [`Event::ResetAll`] event
    fn reset_all_clients(&mut self) {
        self.clients.reset();
    }

    /// Handle a [`Event::SetState`] event
    fn set_client_state(&mut self, id: ClientId, state: ClientState) {
        self.clients
            .set_state(id, state)
            .expect("failed to update client state");
    }

    /// Handle a [`Event::ResetHeartBeat`] event
    fn reset_heartbeat(&mut self, id: ClientId) {
        match self.clients.reset_heartbeat(&id) {
            Ok(()) => {}
            Err(e) => match e {
                // This can happen is we trigger the reset right when
                // the reset occurs. In that case, we don't do
                // anything: the client will be removed when we poll
                // the expiration channel
                HeartBeatResetError::Expired => {}
                // This should not happen
                HeartBeatResetError::ClientNotFound => panic!("{}", e),
                // FIXME: we should remove the node, but our state
                // machine doesn't support that yet, so we just return
                // for now
                HeartBeatResetError::BackPressure => {
                    error!("seems like {} is flooding us with heartbeats", id)
                }
            },
        }
    }

    /// Handle a [`Event::RunSelection`] event
    fn run_selection(&mut self, min_count: u32) {
        if !self.pending_selection.is_empty() {
            return;
        }
        let waiting = self.clients.iter_waiting();
        let selected = self.clients.iter_selected();
        self.pending_selection = self.selector.select(min_count as usize, waiting, selected);
    }

    /// Handle a [`Event::RunAggregation`] event
    fn run_aggregation(&mut self) {
        let result = self.aggregator.aggregate();
    }

    /// Dispatch an [`Event`] to the appropriate handler
    fn dispatch_event(&mut self, event: Event) {
        match event {
            Event::Accept(id) => self.accept_client(id),
            Event::Remove(id) => self.remove_client(id),
            Event::SetState(id, client_state) => self.set_client_state(id, client_state),
            Event::ResetAll => self.reset_all_clients(),
            Event::ResetHeartBeat(id) => self.reset_heartbeat(id),
            Event::RunAggregation => self.run_aggregation(),
            Event::RunSelection(min_count) => self.run_selection(min_count),
        }
    }
}
