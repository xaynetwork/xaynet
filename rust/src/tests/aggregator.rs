use crate::{
    aggregator::service::Service,
    common::client::{ClientId, Credentials, Token},
    tests::lib::{
        aggregator::{ByteAggregator, ServiceHandle},
        enable_logging,
        rpc::coordinator::{Client, MockClient},
    },
};
use bytes::Bytes;
use futures::future;
use tokio::task::JoinHandle;

fn start_service() -> (Client, ServiceHandle<ByteAggregator>, JoinHandle<()>) {
    // Make it easy to debug this test by setting the `TEST_LOGS`
    // environment variable
    enable_logging();

    let aggregator = ByteAggregator::new();
    let rpc_client: Client = MockClient::default().into();

    let (service_handle, service_requests) = ServiceHandle::new();

    let service = Service::new(aggregator, rpc_client.clone(), service_requests);
    let join_handle = tokio::spawn(service);
    (rpc_client, service_handle, join_handle)
}

#[tokio::test]
async fn test_aggregation() {
    let (rpc_client, service_handle, _join_handle) = start_service();

    let client_1_credentials = Credentials(ClientId::new(), Token::new());
    let res = service_handle.select(client_1_credentials).await;
    assert!(res.is_ok());

    let data = Bytes::from_static(b"1111");
    service_handle
        .upload(client_1_credentials, data)
        .await
        .unwrap();

    rpc_client
        .mock()
        .expect_end_training()
        .returning(|_, _, _| future::ready(Ok(())));

    let client_2_credentials = Credentials(ClientId::new(), Token::new());
    let res = service_handle.select(client_2_credentials).await;
    assert!(res.is_ok());

    let data = Bytes::from_static(b"2222");
    service_handle
        .upload(client_2_credentials, data)
        .await
        .unwrap();

    rpc_client
        .mock()
        .expect_end_training()
        .returning(|_, _, _| future::ready(Ok(())));

    let res = service_handle.aggregate().await;
    assert!(res.is_ok());

    let res = service_handle.select(client_1_credentials).await;
    assert!(res.is_ok());

    let res = service_handle.download(client_1_credentials).await;

    let expect = Bytes::from_static(b"11112222");
    assert_eq!(expect[..], res.unwrap()[..]);
}
