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
use super::request::*;
use super::state_machine::*;

use tokio::sync::{mpsc};

use futures::{ready, stream::Stream};

use std::{
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
    clients: Clients,
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
                let status = self.clients.get_state(&id);
                let response = self.state_machine.handle_rendez_vous(id, status);
                sender.send(response);
            }
            Request::HeartBeat((id, sender)) => {
                let response = self
                    .state_machine
                    .handle_heartbeat(id, self.clients.get_state(&id));
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
    fn accept_client(&mut self, id: ClientId) {
        let heartbeat_timer = self.clients.add(id);
        tokio::spawn(heartbeat_timer);
    }

    fn remove_client(&mut self, id: ClientId) {
        // If our implementation is correct, this should never return
        // an error. If it does, our state is invalid, so it is OK to
        // panic.
        self.clients.remove(&id).expect("failed to remove client");
    }

    fn reset_all_clients(&mut self) {
        self.clients.reset();
    }

    fn set_client_state(&mut self, id: ClientId, state: ClientState) {
        self.clients
            .set_state(id, state)
            .expect("failed to update client state");
    }

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
}
