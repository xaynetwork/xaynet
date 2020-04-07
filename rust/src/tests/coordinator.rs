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
        .returning(|_, _| future::ready(Ok(())));

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
        .returning(|_| future::ready(Ok(())));

    service_handle.end_training(id, true).await;
    loop {
        match service_handle.heartbeat(id).await {
            HeartBeatResponse::StandBy => sleep_ms(10).await,
            HeartBeatResponse::Finish => break,
            _ => panic!("expected StandBy or Finish"),
        }
    }
}

#[tokio::test]
async fn dropout_1_participant_during_training() {
    let settings = FederatedLearningSettings {
        rounds: 1,
        participants_ratio: 1.0,
        min_clients: 2,
        heartbeat_timeout: 1,
    };
    let (rpc_client, service_handle, _join_handle) = start_service(settings);

    // Create first client. Since min_clients is 2, the heartbeat response should be `StandBy`.
    let id_1 = service_handle.rendez_vous_accepted().await;
    let hb_resp = service_handle.heartbeat(id_1).await;
    assert_eq!(hb_resp, HeartBeatResponse::StandBy);

    // Create second client.
    let id_2 = service_handle.rendez_vous_accepted().await;

    // Now the the round can start because the requirement of min two clients is fulfilled.
    let round = service_handle.heartbeat_selected(id_1).await;
    assert_eq!(round, 0);
    let round = service_handle.heartbeat_selected(id_2).await;
    assert_eq!(round, 0);

    // Both clients start the training.
    rpc_client
        .mock()
        .expect_select()
        .returning(|_, _| future::ready(Ok(())));
    rpc_client
        .mock()
        .expect_select()
        .returning(|_, _| future::ready(Ok(())));
    let (url, _token) = service_handle.start_training_accepted(id_1).await;
    assert_eq!(&url, AGGREGATOR_URL);
    let (url, _token) = service_handle.start_training_accepted(id_2).await;
    assert_eq!(&url, AGGREGATOR_URL);

    // Here we reset only the heartbeat of the first client and wait until the
    // heartbeat timeout of the second client has been reached.
    service_handle.heartbeat(id_1).await;
    sleep_ms(500).await;
    service_handle.heartbeat(id_1).await;
    sleep_ms(500).await;
    service_handle.heartbeat(id_1).await;
    // The second client should be dropped.

    // The first client finished training.
    service_handle.end_training(id_1, true).await;

    // Create a third client. The third client should be selected by the coordinator and
    // be able to participate in the training session.
    let id_3 = service_handle.rendez_vous_accepted().await;
    let round = service_handle.heartbeat_selected(id_3).await;
    assert_eq!(round, 0);
    rpc_client
        .mock()
        .expect_select()
        .returning(|_, _| future::ready(Ok(())));
    let (url, _token) = service_handle.start_training_accepted(id_3).await;
    assert_eq!(&url, AGGREGATOR_URL);

    // Let's simulate that the second client tries to reconnect to the coordinator.
    let id_2 = service_handle.rendez_vous_accepted().await;

    // The second client should be accepted but not re-selected (for current round)
    // by the coordinator.
    let hb_resp = service_handle.heartbeat(id_2).await;
    assert_eq!(hb_resp, HeartBeatResponse::StandBy);

    // The third client finished training.
    service_handle.end_training(id_3, true).await;

    // Trigger aggregation.
    rpc_client
        .mock()
        .expect_aggregate()
        .returning(|_| future::ready(Ok(())));

    // After the third client finished training, the coordinator should return the heartbeat
    // response `Finish`.
    loop {
        match service_handle.heartbeat(id_3).await {
            HeartBeatResponse::StandBy | HeartBeatResponse::Round(_) => sleep_ms(10).await,
            HeartBeatResponse::Finish => break,
            _ => panic!("expected StandBy, Round or Finish"),
        }
    }
}
