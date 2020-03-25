use crate::{
    coordinator::{core::Service, models::HeartBeatResponse, settings::FederatedLearningSettings},
    tests::lib::{
        coordinator::{MaxSelector, ServiceHandle},
        enable_logging,
        rpc::aggregator::{Client, MockClient},
        sleep_ms,
    },
};
use futures::future;
use tokio::task::JoinHandle;

const AGGREGATOR_URL: &str = "http://localhost:8082";

fn start_service(settings: FederatedLearningSettings) -> (Client, ServiceHandle, JoinHandle<()>) {
    // Make it easy to debug this test by setting the `TEST_LOGS`
    // environment variable
    enable_logging();

    let rpc_client: Client = MockClient::default().into();

    let (service_handle, service_requests) = ServiceHandle::new();

    let service = Service::new(
        MaxSelector,
        settings,
        AGGREGATOR_URL.to_string(),
        rpc_client.clone(),
        service_requests,
    );
    let join_handle = tokio::spawn(service);
    (rpc_client, service_handle, join_handle)
}

/// Test a full cycle with a single round and a single participant.
#[tokio::test]
async fn full_cycle_1_round_1_participant() {
    let settings = FederatedLearningSettings {
        rounds: 1,
        participants_ratio: 1.0,
        min_clients: 1,
        heartbeat_timeout: 10,
    };
    let (rpc_client, service_handle, _join_handle) = start_service(settings);

    let id = service_handle.rendez_vous_accepted().await;
    let round = service_handle.heartbeat_selected(id).await;
    assert_eq!(round, 0);

    rpc_client
        .mock()
        .expect_select()
        .returning(|_, _| future::ready(Ok(Ok(()))));

    let (url, _token) = service_handle.start_training_accepted(id).await;
    assert_eq!(&url, AGGREGATOR_URL);

    // pretend the client trained and sent its weights to the
    // aggregator. The aggregator now sends an end training requests
    // to the coordinator RPC server that we fake with the
    // service_handle. The service should then trigger the aggregation
    // and reject subsequent heartbeats and rendez-vous
    rpc_client
        .mock()
        .expect_aggregate()
        .returning(|_| future::ready(Ok(Ok(()))));

    service_handle.end_training(id, true).await;
    loop {
        match service_handle.heartbeat(id).await {
            HeartBeatResponse::StandBy => sleep_ms(10).await,
            HeartBeatResponse::Finish => break,
            _ => panic!("expected StandBy or Finish"),
        }
    }
}
