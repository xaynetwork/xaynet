use crate::{
    aggregator::rpc::{Client, MockClient},
    common::{
        client::{ClientId, Token},
        logging,
        settings::LoggingSettings,
    },
    coordinator::{
        core::{Selector, Service, ServiceHandle},
        models::{HeartBeatResponse, RendezVousResponse, StartTrainingResponse},
        settings::FederatedLearningSettings,
    },
};
use futures::future;
use std::sync::{Arc, Mutex};
use tracing_subscriber::filter::EnvFilter;

#[tokio::test]
async fn test_rendez_vous_accept() {
    logging::configure(LoggingSettings {
        telemetry: None,
        filter: EnvFilter::try_new("trace").unwrap(),
    });
    let mut rpc_client: Client = MockClient::default().into();
    rpc_client
        .mock()
        .expect_select()
        .returning(|_, _| future::ready(Ok(Ok(()))));
    rpc_client
        .mock()
        .expect_aggregate()
        .returning(|_| future::ready(Ok(Ok(()))));

    let aggregator_url = "http://localhost:8082".to_string();

    let (service_handle, service_requests) = ServiceHandle::new();
    let service_handle = ServiceHandleWrapper(service_handle);

    let service = Service::new(
        MaxSelector,
        FederatedLearningSettings {
            rounds: 1,
            participants_ratio: 1.0,
            min_clients: 1,
            heartbeat_timeout: 10,
        },
        aggregator_url.clone(),
        rpc_client,
        service_requests,
    );
    let _join_handle = tokio::spawn(service);

    let id = service_handle.rendez_vous_accepted().await;
    let round = service_handle.heartbeat_selected(id).await;

    let (url, token) = service_handle.start_training_accepted(id).await;

    assert_eq!(url, aggregator_url);
}

#[derive(Clone)]
struct ServiceHandleWrapper(ServiceHandle);

impl ServiceHandleWrapper {
    async fn rendez_vous_accepted(&self) -> ClientId {
        match self.0.rendez_vous().await.unwrap() {
            RendezVousResponse::Accept(id) => id,
            RendezVousResponse::Reject => panic!("rendez-vous rejected"),
        }
    }
    async fn heartbeat_selected(&self, id: ClientId) -> u32 {
        match self.0.heartbeat(id).await.unwrap() {
            HeartBeatResponse::Round(round) => round,
            resp => panic!("expected HeartBeatResponse::Round(_) got {:?}", resp),
        }
    }
    async fn start_training_accepted(&self, id: ClientId) -> (String, Token) {
        match self.0.start_training(id).await.unwrap() {
            StartTrainingResponse::Accept(url, token) => (url, token),
            StartTrainingResponse::Reject => panic!("start_training rejected"),
        }
    }
}

struct MinSelector;

impl Selector for MinSelector {
    fn select(
        &mut self,
        min_count: usize,
        waiting: impl Iterator<Item = ClientId>,
        selected: impl Iterator<Item = ClientId>,
    ) -> Vec<ClientId> {
        waiting.take(min_count).collect()
    }
}

struct MaxSelector;

impl Selector for MaxSelector {
    fn select(
        &mut self,
        min_count: usize,
        waiting: impl Iterator<Item = ClientId>,
        selected: impl Iterator<Item = ClientId>,
    ) -> Vec<ClientId> {
        waiting.collect()
    }
}

#[derive(Clone, Default)]
struct MutexSelector {
    waiting: Arc<Mutex<Vec<ClientId>>>,
    selected: Arc<Mutex<Vec<ClientId>>>,
    result: Arc<Mutex<Vec<ClientId>>>,
}

impl MutexSelector {
    fn new() -> Self {
        Self::default()
    }
}

impl Selector for MutexSelector {
    fn select(
        &mut self,
        min_count: usize,
        waiting: impl Iterator<Item = ClientId>,
        selected: impl Iterator<Item = ClientId>,
    ) -> Vec<ClientId> {
        {
            let mut self_selected = self.selected.lock().unwrap();
            *self_selected = selected.collect();
        }
        {
            let mut self_waiting = self.waiting.lock().unwrap();
            *self_waiting = waiting.collect();
        }
        let mut self_result = self.result.lock().unwrap();
        self_result.drain(..).collect()
    }
}
