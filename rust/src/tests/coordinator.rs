use crate::{
    aggregator::rpc::Client as MockRpcClient,
    common::client::ClientId,
    coordinator::{
        core::{Selector, Service, ServiceHandle},
        settings::FederatedLearningSettings,
    },
};

#[tokio::test]
async fn test_service() {
    let settings = FederatedLearningSettings {
        rounds: 1,
        participants_ratio: 1.0,
        min_clients: 1,
        heartbeat_timeout: 10,
    };
    let selector = TestSelector(vec![]);
    let aggregator_url = "http://localhost:8082".into();
    let rpc_client = MockRpcClient::default();
    let (service_handle, service_requests) = ServiceHandle::new();

    let join_handle = tokio::spawn(async move {Service::new(
        selector,
        settings,
        aggregator_url,
        rpc_client,
        service_requests,
    )}.await);


}

struct TestSelector(Vec<ClientId>);

impl Selector for TestSelector {
    fn select(
        &mut self,
        min_count: usize,
        waiting: impl Iterator<Item = ClientId>,
        selected: impl Iterator<Item = ClientId>,
    ) -> Vec<ClientId> {
        self.0.clone()
    }
}
