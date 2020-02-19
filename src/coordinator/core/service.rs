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

use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use derive_more::Display;
use futures::{ready, stream::Stream};
use tokio::sync::{mpsc, oneshot};

use crate::{
    common::ClientId,
    coordinator::core::{
        client::{Clients, HeartBeatResetError},
        protocol,
    },
};

pub struct CoordinatorService<A, S, T>
where
    A: Aggregator<T>,
    T: Clone + Debug,
    S: Selector,
{
    requests_rx: mpsc::Receiver<Request<T>>,
    heartbeat_expirations_rx: mpsc::UnboundedReceiver<ClientId>,
    // start_training_expirations_rx: mpsc::UnboundedReceiver<ClientId>,
    // done_training_expirations_rx: mpsc::UnboundedReceiver<ClientId>,
    protocol: protocol::Protocol,
    clients: Clients,
    aggregator: A,
    selector: S,

    pending_selection: Vec<ClientId>,
    global_weights: T,
}

impl<A, S, T> CoordinatorService<A, S, T>
where
    A: Aggregator<T>,
    T: Clone + Debug,
    S: Selector,
{
    pub fn new(
        aggregator: A,
        selector: S,
        global_weights: T,
        config: protocol::CoordinatorConfig,
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
            protocol: protocol::Protocol::new(config),
            pending_selection: Vec::new(),
        };
        let handle = CoordinatorHandle::new(requests_tx);
        (coordinator, handle)
    }

    /// Handle the pending state machine events.
    fn handle_protocol_events(&mut self) {
        while let Some(event) = self.protocol.next_event() {
            self.dispatch_event(event);
        }
        self.sanity_checks();
    }

    /// Handle the incoming requests.
    fn poll_requests(&mut self, cx: &mut Context) -> Poll<()> {
        loop {
            match ready!(Pin::new(&mut self.requests_rx).poll_next(cx)) {
                Some(request) => {
                    self.dispatch_request(request);
                    self.handle_protocol_events();
                }
                None => return Poll::Ready(()),
            }
        }
    }

    fn poll_heartbeat_expirations(&mut self, cx: &mut Context) -> Poll<()> {
        loop {
            match ready!(Pin::new(&mut self.heartbeat_expirations_rx).poll_next(cx)) {
                Some(id) => {
                    let state = self.clients.get_state(&id);
                    self.protocol.hearbeat_timeout(id, state);
                    self.handle_protocol_events();
                }
                None => return Poll::Ready(()),
            }
        }
    }

    /// Handle a request
    fn apply_pending_selection(&mut self) {
        let Self {
            ref mut pending_selection,
            ref mut protocol,
            ref mut clients,
            ..
        } = self;
        if !pending_selection.is_empty() {
            info!("processing pending selection");
            let chunk = pending_selection
                .drain(0..::std::cmp::min(pending_selection.len(), 100))
                .map(|id| (id, clients.get_state(&id)));
            protocol.select(chunk);
            self.handle_protocol_events();
        }
    }

    fn sanity_checks(&self) {
        assert_eq!(self.clients.get_counters(), self.protocol.counters());
    }
}

impl<A, S, T> CoordinatorService<A, S, T>
where
    A: Aggregator<T>,
    T: Clone + Debug,
    S: Selector,
{
    /// Handle a rendez-vous request
    fn rendez_vous(&mut self, req: RequestMessage<RendezVousRequest, RendezVousResponse>) {
        let (_, response_sender) = req;
        let id = ClientId::new();
        // This should be "Unknown" since we just created a
        // new uuid.
        let status = self.clients.get_state(&id);
        let response = match self.protocol.rendez_vous(id, status) {
            protocol::RendezVousResponse::Accept => RendezVousResponse::Accept(id),
            protocol::RendezVousResponse::Reject => RendezVousResponse::Reject,
        };
        response_sender.send(response);
    }

    /// Handle a heartbeat request
    fn heartbeat(&mut self, req: RequestMessage<HeartBeatRequest, HeartBeatResponse>) {
        let (id, response_sender) = req;
        let response = self.protocol.heartbeat(id, self.clients.get_state(&id));
        response_sender.send(response);
    }

    /// Handle a start training request
    fn start_training(
        &mut self,
        req: RequestMessage<StartTrainingRequest, StartTrainingResponse<T>>,
    ) {
        let (id, response_sender) = req;
        let response = match self.protocol.start_training(self.clients.get_state(&id)) {
            protocol::StartTrainingResponse::Accept => {
                StartTrainingPayload::new(self.global_weights.clone()).into()
            }
            protocol::StartTrainingResponse::Reject => StartTrainingResponse::Reject,
        };
        response_sender.send(response);
    }

    /// Handle an end training request
    // FIXME: the end training request should probably made to the
    // aggregator directly, which would then ask the protocol whether
    // it should accept the weights. Right now, handling these
    // requests kinds of break our model where the requests are
    // directly processed by the protocol and events are emitted in
    // response.
    //
    // 1. Client      => Coordinator:   start training request
    // 2. Coordinator => Aggregator:    coordinator sends token for the client
    // 3. Coordinator => Client:        start training response with token + aggregator URL
    // 4. Client      => Aggregator:    end training with weights + token
    // 5. Aggregator  => Coordinator:   client uploaded their results
    //
    // Right now we just check the response returned by the protocol,
    // and pass the weights to the aggregator assuming they are valid.
    fn end_training(&mut self, req: RequestMessage<EndTrainingRequest<T>, EndTrainingResponse>) {
        let ((id, weights), response_sender) = req;
        let response = self.protocol.end_training(id, self.clients.get_state(&id));
        if response == EndTrainingResponse::Accept {
            // FIXME: handle this
            self.aggregator.add_local_result(weights).unwrap();
        }
        response_sender.send(response);
    }

    /// Handle a request
    fn dispatch_request(&mut self, request: Request<T>) {
        match request {
            Request::RendezVous(inner_request) => self.rendez_vous(inner_request),
            Request::HeartBeat(inner_request) => self.heartbeat(inner_request),
            Request::StartTraining(inner_request) => self.start_training(inner_request),
            Request::EndTraining(inner_request) => self.end_training(inner_request),
        }
    }
}

impl<A, S, T> Future for CoordinatorService<A, S, T>
where
    // FIXME: I guess it's OK to require Unpin for the aggregator and
    // the selector ? Unless it is not ?
    A: Aggregator<T> + Unpin,
    S: Selector + Unpin,
    T: Clone + Unpin + Debug,
{
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        trace!("polling CoordinatorService");
        let pin = self.get_mut();

        pin.apply_pending_selection();

        match pin.poll_requests(cx) {
            Poll::Ready(()) => return Poll::Ready(()),
            Poll::Pending => {}
        }

        match pin.poll_heartbeat_expirations(cx) {
            Poll::Ready(()) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<A, S, T> CoordinatorService<A, S, T>
where
    A: Aggregator<T>,
    T: Clone + Debug,
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
    fn set_client_state(&mut self, id: ClientId, state: protocol::ClientState) {
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
        if self.pending_selection.len() >= min_count as usize {
            info!("Event::RunSelection event ignored: pending selection is large enough");
            return;
        }
        let count = min_count as usize - self.pending_selection.len();

        let waiting = self.clients.iter_waiting();
        let selected = self.clients.iter_selected();
        info!(
            "running the selector (selecting at least {} clients)",
            count,
        );
        self.pending_selection = self.selector.select(count as usize, waiting, selected);
        info!(
            "pending selection: {} clients",
            self.pending_selection.len()
        );
    }

    /// Handle a [`Event::RunAggregation`] event
    fn run_aggregation(&mut self) {
        self.global_weights = self.aggregator.aggregate().unwrap();
        info!("aggrgation ran: {:?}", self.global_weights);
    }

    /// Dispatch an [`Event`] to the appropriate handler
    fn dispatch_event(&mut self, event: protocol::Event) {
        use protocol::Event::*;
        info!("handling protocol event {:?}", event);
        match event {
            Accept(id) => self.accept_client(id),
            Remove(id) => self.remove_client(id),
            SetState(id, client_state) => self.set_client_state(id, client_state),
            ResetAll => self.reset_all_clients(),
            ResetHeartBeat(id) => self.reset_heartbeat(id),
            RunAggregation => self.run_aggregation(),
            RunSelection(min_count) => self.run_selection(min_count),
        }
    }
}

/// Error returned when a request fails due to the coordinator having shut down.
#[derive(Debug, Display)]
pub struct RequestError;

impl ::std::error::Error for RequestError {}

pub struct ResponseReceiver<R>(oneshot::Receiver<R>);

pub fn response_channel<R>() -> (ResponseSender<R>, ResponseReceiver<R>) {
    let (tx, rx) = oneshot::channel::<R>();
    (ResponseSender(tx), ResponseReceiver(rx))
}

impl<R> Future for ResponseReceiver<R> {
    type Output = Result<R, RequestError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().0)
            .as_mut()
            .poll(cx)
            .map_err(|_| RequestError)
    }
}

pub struct ResponseSender<R>(oneshot::Sender<R>);

impl<R> ResponseSender<R> {
    pub fn send(self, response: R) {
        self.0.send(response).unwrap_or_else(|_| {
            warn!("failed to send response: receiver shut down");
        })
    }
}

pub type RequestMessage<P, R> = (P, ResponseSender<R>);

// rendez-vous
#[derive(Debug)]
pub struct RendezVousRequest;

#[derive(Debug)]
pub enum RendezVousResponse {
    Accept(ClientId),
    Reject,
}

// heartbeat
pub type HeartBeatRequest = ClientId;
pub use protocol::HeartBeatResponse;

// start training
pub type StartTrainingRequest = ClientId;
pub enum StartTrainingResponse<T> {
    Accept(StartTrainingPayload<T>),
    Reject,
}

pub struct StartTrainingPayload<T> {
    pub global_weights: T,
    // more stuff...
}

impl<T> StartTrainingPayload<T> {
    pub fn new(global_weights: T) -> Self {
        Self { global_weights }
    }
}

impl<T> From<StartTrainingPayload<T>> for StartTrainingResponse<T> {
    fn from(value: StartTrainingPayload<T>) -> Self {
        Self::Accept(value)
    }
}

// end training
pub type EndTrainingRequest<T> = (ClientId, T);
pub use protocol::EndTrainingResponse;

pub enum Request<T> {
    RendezVous(RequestMessage<RendezVousRequest, RendezVousResponse>),
    HeartBeat(RequestMessage<HeartBeatRequest, HeartBeatResponse>),
    StartTraining(RequestMessage<StartTrainingRequest, StartTrainingResponse<T>>),
    EndTraining(RequestMessage<EndTrainingRequest<T>, EndTrainingResponse>),
}

pub struct CoordinatorHandle<T>(mpsc::Sender<Request<T>>);

impl<T> Clone for CoordinatorHandle<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> CoordinatorHandle<T> {
    pub fn new(requests_tx: mpsc::Sender<Request<T>>) -> Self {
        Self(requests_tx)
    }

    pub async fn rendez_vous(&mut self) -> Result<RendezVousResponse, RequestError> {
        let (resp_tx, resp_rx) = response_channel::<RendezVousResponse>();
        let req: Request<T> = Request::RendezVous((RendezVousRequest, resp_tx));
        self.0.send(req).await.map_err(|_| RequestError)?;
        resp_rx.await
    }

    pub async fn heartbeat(&mut self, id: ClientId) -> Result<HeartBeatResponse, RequestError> {
        let (resp_tx, resp_rx) = response_channel::<HeartBeatResponse>();
        let req: Request<T> = Request::HeartBeat((id, resp_tx));
        self.0.send(req).await.map_err(|_| RequestError)?;
        resp_rx.await
    }

    pub async fn start_training(
        &mut self,
        id: ClientId,
    ) -> Result<StartTrainingResponse<T>, RequestError> {
        let (resp_tx, resp_rx) = response_channel::<StartTrainingResponse<T>>();
        let req: Request<T> = Request::StartTraining((id, resp_tx));
        self.0.send(req).await.map_err(|_| RequestError)?;
        resp_rx.await
    }

    pub async fn end_training(
        &mut self,
        id: ClientId,
        weights: T,
    ) -> Result<EndTrainingResponse, RequestError> {
        let (resp_tx, resp_rx) = response_channel::<EndTrainingResponse>();
        let req: Request<T> = Request::EndTraining(((id, weights), resp_tx));
        self.0.send(req).await.map_err(|_| RequestError)?;
        resp_rx.await
    }
}

pub trait Selector {
    fn select(
        &mut self,
        min_count: usize,
        waiting: impl Iterator<Item = ClientId>,
        selected: impl Iterator<Item = ClientId>,
    ) -> Vec<ClientId>;
}

pub trait Aggregator<T> {
    type Error: ::std::error::Error;

    fn add_local_result(&mut self, result: T) -> Result<(), Self::Error>;
    fn aggregate(&mut self) -> Result<T, Self::Error>;
}
