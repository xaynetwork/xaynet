use crate::{
    coordinator::{core::Service, models::HeartBeatResponse, settings::FederatedLearningSettings},
    tests::mocks::{
        coordinator::{MaxSelector, ServiceHandle},
        rpc::aggregator::{Client, MockClient},
    },
};
use futures::future;
use tokio::time::{delay_for, Duration};

#[cfg(logging)]
use crate::common::{logging, settings::LoggingSettings};
#[cfg(logging)]
use tracing_subscriber::filter::EnvFilter;

#[tokio::test]
async fn test_rendez_vous_accept() {
    #[cfg(logging)]
    logging::configure(LoggingSettings {
        telemetry: None,
        filter: EnvFilter::try_new("trace").unwrap(),
    });

    let rpc_client: Client = MockClient::default().into();
    let aggregator_url = "http://localhost:8082".to_string();

    let (service_handle, service_requests) = ServiceHandle::new();

    let service = Service::new(
        MaxSelector,
        FederatedLearningSettings {
            rounds: 1,
            participants_ratio: 1.0,
            min_clients: 1,
            heartbeat_timeout: 10,
        },
        aggregator_url.clone(),
        rpc_client.clone(),
        service_requests,
    );
    let _join_handle = tokio::spawn(service);

    let id = service_handle.rendez_vous_accepted().await;
    let round = service_handle.heartbeat_selected(id).await;
    assert_eq!(round, 0);

    rpc_client
        .mock()
        .expect_select()
        .returning(|_, _| future::ready(Ok(Ok(()))));

    let (url, _token) = service_handle.start_training_accepted(id).await;
    assert_eq!(url, aggregator_url);

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

async fn sleep_ms(ms: u64) {
    delay_for(Duration::from_millis(ms)).await
}
