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

#![allow(dead_code)]
#![allow(unused_imports)]
use crate::coordinator::{Aggregator, Selector};

use super::client::*;
use super::heartbeat::*;
use super::request::*;
use super::state_machine::*;

use tokio::sync::{mpsc, oneshot};

use futures::{ready, stream::Stream};

use std::{
    collections::{HashMap, HashSet},
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(10);
const HEARTBEAT_TIME: Duration = Duration::from_secs(5);

pub struct CoordinatorService<A, S, T>
where
    A: Aggregator<T>,
    S: Selector,
{
    requests: mpsc::Receiver<Request>,
    heartbeat_expirations_rx: mpsc::UnboundedReceiver<ClientId>,
    // start_training_expirations_rx: mpsc::UnboundedReceiver<ClientId>,
    // done_training_expirations_rx: mpsc::UnboundedReceiver<ClientId>,
    state_machine: StateMachine,
    client: Clients,
    aggregator: A,
    selector: S,
    _phantom: PhantomData<T>,
}

impl<A, S, T> CoordinatorService<A, S, T>
where
    A: Aggregator<T>,
    S: Selector,
{
    /// Handle the pending state machine events.
    fn handle_state_machine_events(&mut self) {
        while let Some(event) = self.state_machine.next_event() {
            <Self as StateMachineEventHandler>::dispatch_event(self, event);
        }
    }

    /// Handle the incoming requests.
    fn poll_requests(&mut self, cx: &mut Context) -> Poll<Option<()>> {
        loop {
            match ready!(Pin::new(&mut self.requests).poll_next(cx)) {
                Some(request) => {
                    self.handle_request(request);
                    self.handle_state_machine_events();
                }
                None => return Poll::Ready(None),
            }
        }
    }

    fn handle_request(&mut self, request: Request) {
        match request {
            Request::RendezVous((opt_id, sender)) => {
                // There can be no ID provided, if this is the first
                // time the client sends a rendez-vous request. In
                // that case we generate one.
                let id = opt_id.unwrap_or_else(|| ClientId::new());
                // Find the client status by ID, defaulting to
                // Unknown.
                let status = self.get_client_status(&id);
                let response = self.state_machine.handle_rendez_vous(id, status);
                sender.send(response);
            }
            Request::Heartbeat((id, sender)) => {
                let response = self
                    .state_machine
                    .handle_heartbeat(id, self.get_client_status(&id));
                sender.send(response);
            }
        }
        self.handle_state_machine_events()
    }
}

impl<A, S, T> Future for CoordinatorService<A, S, T>
where
    // FIXME: I guess it's OK to require Unpin for the aggregator and
    // the selector ? Unless it is not ?
    A: Aggregator<T> + Unpin,
    S: Selector + Unpin,
    T: Unpin,
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

        Poll::Pending
    }
}

// We must be particularly careful in handling state machine events
// because we have to keep our clients state in sync with the state
// machine: inconsistencies between these two elements means that the
// coordinator behavior is incorrect. To prevent that, these methods
// contains a lot of assert! which will crash the coordinator when the
// invariants we expect to be held are violated.
//
// This is a design choice where we privilege correctness over
// robustness.
impl<A, S, T> StateMachineEventHandler for CoordinatorService<A, S, T>
where
    A: Aggregator<T>,
    S: Selector,
{
    /// Create a new client and start its heartbeat timer.
    fn accept_client(&mut self, id: ClientId) {
        let heartbeat_expirations_tx = self.heartbeat_expirations_tx.clone();
        self.clients.new(id, heartbeat_reset_tx);
        tokio::spawn(heartbeat_timer);
    }

    /// Remove the given client
    fn handle_remove_node_event(&mut self, id: ClientId) {
        let client = self.clients.remove(&id).expect("cannot remove client {}: not found");

    }

    fn handle_reset_heartbeat_timer_event(&mut self, id: ClientId) {
        if let Some(client) = self.clients.get_mut(&id) {
            if client.reset_heartbeat_timer(HEARTBEAT_TIMEOUT).is_err() {
                self.clients.remove(&id);
            }
            return;
        }
        debug_assert!(
            false,
            "could not reset heartbeat timer for client {}: not found",
            id
        );
        debug!(
            "could not reset heartbeat timer for client {}: not found",
            id
        );
    }

    fn handle_update_client_status_event(&mut self, id: ClientId, status: ClientStatus) {
        match (self.get_client_status(&id), status) {
            // It is an error to update the state of a client if it is
            // already in that state. In itself, it has no
            // consequence, but it should not happen so something is
            // wrong.
            (current, new) if current == new => {
                panic!("updating client status with the same status {}", new);
            }
            // If the current client status is DoneAndInactive, we should
            // already have it in `self.done_but_dead_clients` and we
            // need to:
            //
            //  - remove it from `self.done_but_dead_clients`
            //  - re-create a new client, spawn its timers, and add it
            //    to `self.clients`
            (ClientStatus::DoneAndInactive, _) => {
                assert!(
                    self.done_but_dead_clients.contains(&id),
                    "DoneAndInactive client not found"
                );
                self.new_client(id, Some(status));
            }
            // If the client is unknown, we should not have it
            (ClientStatus::Unknown, _) => {
                assert!(
                    status != ClientStatus::DoneAndInactive,
                    "Unknown node cannot have status DoneAndInactive"
                );
                assert!(
                    !self.clients.contains_key(&id),
                    "Unknown node already exists"
                );
                assert!(
                    !self.done_but_dead_clients.contains(&id),
                    "Unknown node already exists as DoneAndInactive"
                );
            }
        }
    }
}
