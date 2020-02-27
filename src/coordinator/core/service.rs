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
    time::Duration,
};

use derive_more::Display;
use futures::{ready, stream::Stream};
use tokio::{
    net::ToSocketAddrs,
    sync::{mpsc, oneshot},
};

use crate::{
    aggregator,
    common::{ClientId, Token},
    coordinator::{
        core::{
            client::{Clients, HeartBeatResetError},
            protocol,
        },
        models::{HeartBeatResponse, RendezVousResponse, StartTrainingResponse},
        rpc,
        settings::FederatedLearningSettings,
    },
};

use tarpc::context::current as rpc_context;

struct AggregationFuture(Pin<Box<dyn Future<Output = Result<(), ()>>>>);

impl Future for AggregationFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().0.as_mut().poll(cx)
    }
}

impl AggregationFuture {
    fn new(rpc_client: aggregator::rpc::Client) -> Self {
        Self(Box::pin(async move { run_aggregation(rpc_client).await }))
    }
}

async fn run_aggregation(mut rpc_client: aggregator::rpc::Client) -> Result<(), ()> {
    match rpc_client.aggregate(rpc_context()).await {
        Ok(result) => result.map_err(|()| warn!("aggregator failed to perform aggregation")),
        Err(e) => {
            warn!("failed to send aggregation request: io error: {}", e);
            Err(())
        }
    }
}

pub struct CoordinatorService<S>
where
    S: Selector,
{
    /// Incoming requests from the clients
    requests_rx: mpsc::Receiver<Request>,

    /// HeartBeat timers that expired
    // FIXME: we should have timeouts for start training and
    // end training as well:
    //
    // start_training_expirations_rx: mpsc::UnboundedReceiver<ClientId>,
    // done_training_expirations_rx: mpsc::UnboundedReceiver<ClientId>,
    heartbeat_expirations_rx: mpsc::UnboundedReceiver<ClientId>,

    /// Protocol state machine
    protocol: protocol::Protocol,

    /// Clients states
    clients: Clients,

    /// Type that performs the selection
    selector: S,

    /// URL of the aggregator for clients to download/upload model weights
    aggregator_url: String,

    /// RPC client for the aggregator service. The RPC client
    /// automatically tried to reconnect when the connection shuts
    /// down, so after the initial connection, it is always available.
    aggregator_rpc: Option<aggregator::rpc::Client>,

    /// Future for the initial connection to the aggregator. Once this
    /// future finishes, it returns a client that is stored in the
    /// `aggregator_rpc` attribute.
    aggregator_rpc_connection: Option<aggregator::rpc::ConnectFuture>,

    /// Future that resolve when the aggregator finishes the
    /// aggregation.
    aggregation_future: Option<AggregationFuture>,

    /// Channel for receiving the RPC requests from the aggregator
    rpc_requests: rpc::RequestReceiver,

    /// IDs of the clients that the selector picked, but that the
    /// protocol doesn't know yet. The reason for this pending
    /// selection is to apply the selection by small chunks instead of
    /// all at once, in order to not block the executor, if a huge
    /// amount of clients are selected.
    pending_selection: Vec<ClientId>,
}

impl<S> CoordinatorService<S>
where
    S: Selector,
{
    pub fn new<
        T: Clone + ToSocketAddrs + Send + Sync + 'static + Unpin,
        U: ToSocketAddrs + Send + Sync + 'static,
    >(
        selector: S,
        fl_settings: FederatedLearningSettings,
        aggregator_url: String,
        rpc_listen_addr: U,
        aggregator_rpc_addr: T,
    ) -> (Self, CoordinatorHandle) {
        let (requests_tx, requests_rx) = mpsc::channel(2048);
        let (heartbeat_expirations_tx, heartbeat_expirations_rx) = mpsc::unbounded_channel();

        let heartbeat_timeout = Duration::from_secs(fl_settings.heartbeat_timeout);
        let rpc_requests = rpc::run(rpc_listen_addr);
        let coordinator = Self {
            selector,
            heartbeat_expirations_rx,
            requests_rx,
            clients: Clients::new(heartbeat_expirations_tx, heartbeat_timeout),
            protocol: protocol::Protocol::new(fl_settings),
            pending_selection: Vec::new(),
            aggregator_rpc: None,
            aggregator_rpc_connection: Some(aggregator::rpc::ConnectFuture::new(
                aggregator_rpc_addr,
            )),
            aggregation_future: None,
            aggregator_url,
            rpc_requests,
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
        trace!("polling requests");
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

    /// If there is an aggregation request running, poll the
    /// corresponding future
    fn poll_aggregation(&mut self, cx: &mut Context) -> Poll<()> {
        if let Some(ref mut fut) = self.aggregation_future {
            match ready!(Pin::new(fut).poll(cx)) {
                // FIXME: there are lots of things to think about
                // when aggregation has failed. Currently the
                // protocol just doesn't increment the round
                // number. But we also need to make sure that the
                // aggregators is reset and that the global weights
                // are not updated.
                Ok(()) => self.protocol.end_aggregation(true),
                Err(()) => self.protocol.end_aggregation(false),
            }
            self.aggregation_future = None;
        }
        Poll::Pending
    }

    fn poll_heartbeat_expirations(&mut self, cx: &mut Context) -> Poll<()> {
        loop {
            match ready!(Pin::new(&mut self.heartbeat_expirations_rx).poll_next(cx)) {
                Some(id) => {
                    debug!("heartbeat expired: {}", id);
                    let state = self.clients.get_state(&id);
                    self.protocol.hearbeat_timeout(id, state);
                    self.handle_protocol_events();
                }
                None => return Poll::Ready(()),
            }
        }
    }

    fn poll_rpc_requests(&mut self, cx: &mut Context) -> Poll<()> {
        loop {
            match ready!(Pin::new(&mut self.rpc_requests).poll_next(cx)) {
                Some((id, success)) => {
                    let state = self.clients.get_state(&id);
                    self.protocol.end_training(id, success, state);
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
            debug!("processing pending selection");
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

impl<S> CoordinatorService<S>
where
    S: Selector,
{
    /// Handle a rendez-vous request
    fn rendez_vous(&mut self, req: RequestMessage<(), RendezVousResponse>) {
        debug!("handling rendez-vous request");
        let (_, response_tx) = req;
        let id = ClientId::new();
        // This should be "Unknown" since we just created a
        // new uuid.
        let status = self.clients.get_state(&id);
        let response = match self.protocol.rendez_vous(id, status) {
            protocol::RendezVousResponse::Accept => RendezVousResponse::Accept(id),
            protocol::RendezVousResponse::Reject => RendezVousResponse::Reject,
        };
        response_tx.send(response);
    }

    /// Handle a heartbeat request
    fn heartbeat(&mut self, req: RequestMessage<ClientId, HeartBeatResponse>) {
        debug!("handling heartbeat request");
        let (id, response_tx) = req;
        let response = self.protocol.heartbeat(id, self.clients.get_state(&id));
        response_tx.send(response);
    }

    /// Handle a start training request
    fn start_training(&mut self, req: RequestMessage<ClientId, StartTrainingResponse>) {
        debug!("handling start training request");
        let (id, response_tx) = req;
        match self.protocol.start_training(self.clients.get_state(&id)) {
            protocol::StartTrainingResponse::Reject => {
                response_tx.send(StartTrainingResponse::Reject)
            }
            protocol::StartTrainingResponse::Accept => {
                if !self.aggregator_rpc.is_some() {
                    // FIXME: like above, we should return an error
                    // instead of just dropping the response channel.
                    warn!("no connection to the aggregator, cannot send token");
                    return;
                }

                let mut rpc_client = self.aggregator_rpc.clone().unwrap();
                let url = self.aggregator_url.clone();

                tokio::spawn(async move {
                    let token = Token::new();
                    // FIXME: upon RPC failure or if the aggregator
                    // returns an error, we currently just drop the
                    // response channel. For the sake of clarity,
                    // maybe we should probably return a proper error
                    // instead.
                    match rpc_client.select(rpc_context(), id, token).await {
                        Ok(result) => {
                            if result.is_ok() {
                                response_tx.send(StartTrainingResponse::Accept(url, token));
                            }
                        }
                        Err(e) => {
                            warn!("failed to send start training request: io error: {}", e);
                        }
                    }
                });
            }
        }
    }

    /// Handle a request
    fn dispatch_request(&mut self, request: Request) {
        info!("handling request: {:?}", request);
        match request {
            Request::RendezVous(inner_request) => self.rendez_vous(inner_request),
            Request::HeartBeat(inner_request) => self.heartbeat(inner_request),
            Request::StartTraining(inner_request) => self.start_training(inner_request),
        }
    }
}

impl<S> Future for CoordinatorService<S>
where
    S: Selector + Unpin,
{
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        trace!("polling CoordinatorService");
        let pin = self.get_mut();

        // This only runs when the coordinator starts
        if let Some(ref mut connection) = pin.aggregator_rpc_connection {
            match Pin::new(connection).poll(cx) {
                Poll::Ready(Ok(client)) => {
                    pin.aggregator_rpc = Some(client);
                    pin.aggregator_rpc_connection = None;
                }
                Poll::Ready(Err(e)) => {
                    error!("failed to connect RPC client: {}", e);
                    return Poll::Ready(());
                }
                _ => {}
            }
        }

        pin.apply_pending_selection();

        match pin.poll_requests(cx) {
            Poll::Ready(()) => return Poll::Ready(()),
            Poll::Pending => {}
        }

        match pin.poll_heartbeat_expirations(cx) {
            Poll::Ready(()) => return Poll::Ready(()),
            Poll::Pending => {}
        }

        match pin.poll_rpc_requests(cx) {
            Poll::Ready(()) => return Poll::Ready(()),
            Poll::Pending => {}
        }

        match pin.poll_aggregation(cx) {
            Poll::Ready(()) => return Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S> CoordinatorService<S>
where
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
        match self.aggregator_rpc {
            Some(ref client) => {
                self.aggregation_future = Some(AggregationFuture::new(client.clone()))
            }
            None => self.protocol.end_aggregation(false),
        }
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

pub enum Request {
    RendezVous(RequestMessage<(), RendezVousResponse>),
    HeartBeat(RequestMessage<ClientId, HeartBeatResponse>),
    StartTraining(RequestMessage<ClientId, StartTrainingResponse>),
}

pub struct CoordinatorHandle(mpsc::Sender<Request>);

impl Clone for CoordinatorHandle {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl CoordinatorHandle {
    pub fn new(requests_tx: mpsc::Sender<Request>) -> Self {
        Self(requests_tx)
    }

    pub async fn rendez_vous(&mut self) -> Result<RendezVousResponse, RequestError> {
        let (resp_tx, resp_rx) = response_channel::<RendezVousResponse>();
        let req: Request = Request::RendezVous(((), resp_tx));
        self.0.send(req).await.map_err(|_| RequestError)?;
        resp_rx.await
    }

    pub async fn heartbeat(&mut self, id: ClientId) -> Result<HeartBeatResponse, RequestError> {
        trace!("notifying service about hearbeat from {}", id);
        let (resp_tx, resp_rx) = response_channel::<HeartBeatResponse>();
        let req: Request = Request::HeartBeat((id, resp_tx));
        self.0.send(req).await.map_err(|_| {
            error!("failed to send heartbeat request to CoordinatorService");
            RequestError
        })?;
        resp_rx.await
    }

    pub async fn start_training(
        &mut self,
        id: ClientId,
    ) -> Result<StartTrainingResponse, RequestError> {
        let (resp_tx, resp_rx) = response_channel::<StartTrainingResponse>();
        let req: Request = Request::StartTraining((id, resp_tx));
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
