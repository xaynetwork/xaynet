#[cfg(feature = "influx_metrics")]
use crate::common::metric_store::influxdb::{CountersMeasurement, Measurement, RoundMeasurement};
use crate::{
    aggregator,
    common::client::{ClientId, Credentials, Token},
    coordinator::{
        core::{
            client::{Clients, HeartBeatResetError},
            protocol,
        },
        models::{HeartBeatResponse, RendezVousResponse, StartTrainingResponse},
        settings::FederatedLearningSettings,
    },
};
use derive_more::From;
use futures::{ready, stream::Stream};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tarpc::context::current as rpc_context;
use tokio::{
    stream::StreamExt,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
};

struct AggregationFuture(Pin<Box<dyn Future<Output = Result<(), ()>> + Send>>);

impl Future for AggregationFuture {
    type Output = Result<(), ()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().0.as_mut().poll(cx)
    }
}

impl AggregationFuture {
    fn new(mut rpc_client: aggregator::rpc::Client) -> Self {
        Self(Box::pin(async move {
            rpc_client.aggregate(rpc_context()).await.map_err(|e| {
                error!(error=%e, "failed to perform aggregation");
            })
        }))
    }
}

pub struct Service<S>
where
    S: Selector,
{
    /// HeartBeat timers that expired
    // FIXME: we should have timeouts for start training and
    // end training as well:
    //
    // start_training_expirations_rx: UnboundedReceiver<ClientId>,
    // done_training_expirations_rx: UnboundedReceiver<ClientId>,
    heartbeat_expirations_rx: UnboundedReceiver<ClientId>,

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
    rpc_client: aggregator::rpc::Client,

    /// Future that resolve when the aggregator finishes the
    /// aggregation.
    aggregation_future: Option<AggregationFuture>,

    requests: ServiceRequests,

    /// IDs of the clients that the selector picked, but that the
    /// protocol doesn't know yet. The reason for this pending
    /// selection is to apply the selection by small chunks instead of
    /// all at once, in order to not block the executor, if a huge
    /// amount of clients are selected.
    pending_selection: Vec<ClientId>,

    #[cfg(feature = "influx_metrics")]
    ///Metric Store
    metrics_tx: Option<UnboundedSender<Measurement>>,
}

impl<S> Service<S>
where
    S: Selector,
{
    pub fn new(
        selector: S,
        fl_settings: FederatedLearningSettings,
        aggregator_url: String,
        rpc_client: aggregator::rpc::Client,
        requests: ServiceRequests,
        #[cfg(feature = "influx_metrics")] metrics_tx: Option<UnboundedSender<Measurement>>,
    ) -> Self {
        let (heartbeat_expirations_tx, heartbeat_expirations_rx) = unbounded_channel();

        let heartbeat_timeout = Duration::from_secs(fl_settings.heartbeat_timeout);
        Self {
            selector,
            heartbeat_expirations_rx,
            clients: Clients::new(heartbeat_expirations_tx, heartbeat_timeout),
            protocol: protocol::Protocol::new(fl_settings),
            pending_selection: Vec::new(),
            rpc_client,
            aggregation_future: None,
            aggregator_url,
            requests,
            #[cfg(feature = "influx_metrics")]
            metrics_tx,
        }
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
            match ready!(Pin::new(&mut self.requests).poll_next(cx)) {
                Some(request) => {
                    self.handle_request(request);
                    self.handle_protocol_events();
                }
                None => return Poll::Ready(()),
            }
        }
    }

    /// Handle a request
    fn handle_request(&mut self, request: Request) {
        match request {
            Request::RendezVous(req) => self.handle_rendez_vous_request(req),
            Request::HeartBeat(req) => self.handle_heartbeat_request(req),
            Request::StartTraining(req) => self.handle_start_training_request(req),
            Request::EndTraining(req) => self.handle_end_training_request(req),
        }
    }
    /// Handle a rendez-vous request
    fn handle_rendez_vous_request(&mut self, req: RendezVousRequest) {
        debug!("handling rendez-vous request");
        let RendezVousRequest { response_tx } = req;
        let id = ClientId::new();
        // This should be "Unknown" since we just created a
        // new uuid.
        let status = self.clients.get_state(&id);
        let response = match self.protocol.rendez_vous(id, status) {
            protocol::RendezVousResponse::Accept => RendezVousResponse::Accept(id),
            protocol::RendezVousResponse::Reject => RendezVousResponse::Reject,
        };
        if response_tx.send(response).is_err() {
            warn!("failed to send response back: channel closed");
        }
    }

    /// Handle a heartbeat request
    fn handle_heartbeat_request(&mut self, req: HeartBeatRequest) {
        debug!("handling heartbeat request");
        let HeartBeatRequest { id, response_tx } = req;
        let response = self.protocol.heartbeat(id, self.clients.get_state(&id));

        if response_tx.send(response).is_err() {
            warn!("failed to send response back: channel closed");
        }
    }

    /// Handle a start training request
    fn handle_start_training_request(&mut self, req: StartTrainingRequest) {
        debug!("handling start training request");
        let StartTrainingRequest { id, response_tx } = req;
        match self.protocol.start_training(self.clients.get_state(&id)) {
            protocol::StartTrainingResponse::Reject => {
                if response_tx.send(StartTrainingResponse::Reject).is_err() {
                    warn!("failed to send response back: channel closed");
                }
            }
            protocol::StartTrainingResponse::Accept => {
                let mut rpc_client = self.rpc_client.clone();
                let url = self.aggregator_url.clone();

                tokio::spawn(async move {
                    let token = Token::new();
                    let credentials = Credentials(id, token);
                    // FIXME: upon RPC failure or if the aggregator
                    // returns an error, we currently just drop the
                    // response channel. For the sake of clarity,
                    // maybe we should probably return a proper error
                    // instead.
                    match rpc_client.select(rpc_context(), credentials).await {
                        Ok(()) => {
                            if response_tx
                                .send(StartTrainingResponse::Accept(url, token))
                                .is_err()
                            {
                                warn!("failed to send response back: channel closed");
                            };
                        }
                        Err(e) => {
                            warn!(error=%e, "select request failed");
                        }
                    }
                });
            }
        }
    }

    /// Handle a start training request
    fn handle_end_training_request(&mut self, req: EndTrainingRequest) {
        debug!("handling end training request");
        let EndTrainingRequest { id, success } = req;
        let state = self.clients.get_state(&id);
        self.protocol.end_training(id, success, state);
    }

    /// If there is an aggregation request running, poll the
    /// corresponding future
    fn poll_aggregation(&mut self, cx: &mut Context) -> Poll<()> {
        if let Some(ref mut fut) = self.aggregation_future {
            trace!("polling aggregation future");
            match ready!(Pin::new(fut).poll(cx)) {
                // FIXME: there are lots of things to think about
                // when aggregation has failed. Currently the
                // protocol just doesn't increment the round
                // number. But we also need to make sure that the
                // aggregators is reset and that the global weights
                // are not updated.
                Ok(()) => {
                    info!("aggregation finished successfully");
                    self.protocol.end_aggregation(true);
                }
                Err(()) => {
                    info!("aggregation failed");
                    self.protocol.end_aggregation(false);
                }
            }
            self.aggregation_future = None;
            self.handle_protocol_events();
        }
        Poll::Pending
    }

    fn poll_heartbeat_expirations(&mut self, cx: &mut Context) -> Poll<()> {
        loop {
            match ready!(Pin::new(&mut self.heartbeat_expirations_rx).poll_next(cx)) {
                Some(id) => {
                    debug!("heartbeat expired: {}", id);
                    let state = self.clients.get_state(&id);
                    self.protocol.heartbeat_timeout(id, state);
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

impl<S> Future for Service<S>
where
    S: Selector + Unpin,
{
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        trace!("polling Service");
        let pin = self.get_mut();

        pin.apply_pending_selection();

        match pin.poll_requests(cx) {
            Poll::Ready(()) => return Poll::Ready(()),
            Poll::Pending => {}
        }

        match pin.poll_heartbeat_expirations(cx) {
            Poll::Ready(()) => return Poll::Ready(()),
            Poll::Pending => {}
        }

        match pin.poll_aggregation(cx) {
            Poll::Ready(()) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S> Service<S>
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
        self.aggregation_future = Some(AggregationFuture::new(self.rpc_client.clone()))
    }

    #[cfg(feature = "influx_metrics")]
    fn write_counter_metrics(&self) {
        self.metrics_tx.as_ref().map(|tx| {
            let _ = tx.send(
                CountersMeasurement::new(
                    self.protocol.counters().selected,
                    self.protocol.counters().waiting,
                    self.protocol.counters().done,
                    self.protocol.counters().done_and_inactive,
                    self.protocol.counters().ignored,
                )
                .into(),
            );
        });
    }

    #[cfg(feature = "influx_metrics")]
    fn write_round_metric(&self, round: u32) {
        self.metrics_tx.as_ref().map(|tx| {
            let _ = tx.send(RoundMeasurement::new(round).into());
        });
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
            EndRound(_) => (),
        }

        #[cfg(feature = "influx_metrics")]
        match event {
            Accept(_) | Remove(_) | SetState(_, _) => self.write_counter_metrics(),
            EndRound(round) => self.write_round_metric(round),
            _ => (),
        }
    }
}

#[derive(Debug)]
pub struct RequestError;

pub struct ServiceRequests(Pin<Box<dyn Stream<Item = Request> + Send>>);

impl Stream for ServiceRequests {
    type Item = Request;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        trace!("polling ServiceRequests");
        self.0.as_mut().poll_next(cx)
    }
}

impl ServiceRequests {
    fn new(
        rendez_vous: UnboundedReceiver<RendezVousRequest>,
        start_training: UnboundedReceiver<StartTrainingRequest>,
        end_training: UnboundedReceiver<EndTrainingRequest>,
        heartbeat: UnboundedReceiver<HeartBeatRequest>,
    ) -> Self {
        let stream = rendez_vous
            .map(Request::from)
            .merge(start_training.map(Request::from))
            .merge(end_training.map(Request::from))
            .merge(heartbeat.map(Request::from));
        Self(Box::pin(stream))
    }
}

#[derive(From)]
pub enum Request {
    RendezVous(RendezVousRequest),
    HeartBeat(HeartBeatRequest),
    StartTraining(StartTrainingRequest),
    EndTraining(EndTrainingRequest),
}

#[derive(From)]
pub struct RendezVousRequest {
    response_tx: oneshot::Sender<RendezVousResponse>,
}

#[derive(From)]
pub struct HeartBeatRequest {
    id: ClientId,
    response_tx: oneshot::Sender<HeartBeatResponse>,
}

#[derive(From)]
pub struct StartTrainingRequest {
    id: ClientId,
    response_tx: oneshot::Sender<StartTrainingResponse>,
}

#[derive(From)]
pub struct EndTrainingRequest {
    id: ClientId,
    success: bool,
}

#[derive(Clone)]
pub struct ServiceHandle {
    rendez_vous: UnboundedSender<RendezVousRequest>,
    start_training: UnboundedSender<StartTrainingRequest>,
    end_training: UnboundedSender<EndTrainingRequest>,
    heartbeat: UnboundedSender<HeartBeatRequest>,
}

impl ServiceHandle {
    pub fn new() -> (Self, ServiceRequests) {
        let (rendez_vous_tx, rendez_vous_rx) = unbounded_channel::<RendezVousRequest>();
        let (start_training_tx, start_training_rx) = unbounded_channel::<StartTrainingRequest>();
        let (end_training_tx, end_training_rx) = unbounded_channel::<EndTrainingRequest>();
        let (heartbeat_tx, heartbeat_rx) = unbounded_channel::<HeartBeatRequest>();

        let handle = Self {
            rendez_vous: rendez_vous_tx,
            start_training: start_training_tx,
            heartbeat: heartbeat_tx,
            end_training: end_training_tx,
        };
        let service_requests = ServiceRequests::new(
            rendez_vous_rx,
            start_training_rx,
            end_training_rx,
            heartbeat_rx,
        );
        (handle, service_requests)
    }
    pub async fn rendez_vous(&self) -> Result<RendezVousResponse, RequestError> {
        let (tx, rx) = oneshot::channel();
        Self::send_request(RendezVousRequest::from(tx), &self.rendez_vous);
        rx.await.map_err(|_| {
            warn!("could not receive response: channel closed");
            RequestError
        })
    }

    pub async fn heartbeat(&self, id: ClientId) -> Result<HeartBeatResponse, RequestError> {
        let (tx, rx) = oneshot::channel();
        Self::send_request(HeartBeatRequest::from((id, tx)), &self.heartbeat);
        rx.await.map_err(|_| {
            warn!("could not receive response: channel closed");
            RequestError
        })
    }

    pub async fn start_training(
        &self,
        id: ClientId,
    ) -> Result<StartTrainingResponse, RequestError> {
        let (tx, rx) = oneshot::channel();
        Self::send_request(StartTrainingRequest::from((id, tx)), &self.start_training);
        rx.await.map_err(|_| {
            warn!("could not receive response: channel closed");
            RequestError
        })
    }

    pub async fn end_training(&self, id: ClientId, success: bool) {
        Self::send_request(EndTrainingRequest::from((id, success)), &self.end_training);
    }

    fn send_request<P>(payload: P, chan: &UnboundedSender<P>) {
        trace!("send request to the service");
        if chan.send(payload).is_err() {
            warn!("failed to send request: channel closed");
            return;
        }
        trace!("request sent");
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
