use crate::{
    common::client::{ClientId, Token},
    coordinator::{
        core::{Selector, ServiceHandle as InnerServiceHandle, ServiceRequests},
        models::{HeartBeatResponse, RendezVousResponse, StartTrainingResponse},
    },
};

/// A selector that always select all the participants currently
/// waiting.
pub struct MaxSelector;

impl Selector for MaxSelector {
    fn select(
        &mut self,
        _min_count: usize,
        waiting: impl Iterator<Item = ClientId>,
        _selected: impl Iterator<Item = ClientId>,
    ) -> Vec<ClientId> {
        waiting.collect()
    }
}

/// A wrapper for [`coordinator::core::ServiceHandle`] with some
/// convenience methods that reduce boilerplate in the tests.
#[derive(Clone)]
pub struct ServiceHandle(InnerServiceHandle);

impl ServiceHandle {
    /// Create a new `ServiceHandle`. The returned [`ServiceRequets`]
    /// can be passed directly to [`Service::new`].
    pub fn new() -> (Self, ServiceRequests) {
        let (inner, requests) = InnerServiceHandle::new();
        (Self(inner), requests)
    }

    /// Send a rendez-vous request assuming it's going to be accepted
    /// and return the client ID given by the coordinator service.
    ///
    /// # Panic
    ///
    /// This method panics if the service fails to answer the request
    /// of if the rendez-vous request is rejected.
    pub async fn rendez_vous_accepted(&self) -> ClientId {
        match self.0.rendez_vous().await.unwrap() {
            RendezVousResponse::Accept(id) => id,
            RendezVousResponse::Reject => panic!("rendez-vous rejected"),
        }
    }

    /// Send a heartbeat, assuming the response will be a
    /// [`HeartBeatResponse::Round`].
    ///
    /// # Panic
    ///
    /// This method panics if the service fails to answer the request,
    /// or if the heartbeat response is not
    /// `HeartBeatResponse::Round`.
    pub async fn heartbeat_selected(&self, id: ClientId) -> u32 {
        match self.0.heartbeat(id).await.unwrap() {
            HeartBeatResponse::Round(round) => round,
            resp => panic!("expected HeartBeatResponse::Round(_) got {:?}", resp),
        }
    }

    /// Send a heartbeat, assuming the response will be a
    /// [`HeartBeatResponse::Round`].
    ///
    /// # Panic
    ///
    /// This method panics if the service fails to answer the request
    /// or if the heartbeat response is not
    /// `HeartBeatResponse::Round`.
    pub async fn heartbeat(&self, id: ClientId) -> HeartBeatResponse {
        self.0.heartbeat(id).await.unwrap()
    }

    /// Send a start training request, assuming it will be accepted.
    ///
    /// # Panic
    ///
    /// This method panics if the service fails to answer the request
    /// or if it rejects it.
    pub async fn start_training_accepted(&self, id: ClientId) -> (String, Token) {
        match self.0.start_training(id).await.unwrap() {
            StartTrainingResponse::Accept(url, token) => (url, token),
            StartTrainingResponse::Reject => panic!("start_training rejected"),
        }
    }

    /// Send an training request
    pub async fn end_training(&self, id: ClientId, success: bool) {
        self.0.end_training(id, success).await
    }
}
