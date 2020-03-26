use crate::{
    common::client::{ClientId, Token},
    coordinator::{
        core::{Selector, ServiceHandle as InnerServiceHandle, ServiceRequests},
        models::{HeartBeatResponse, RendezVousResponse, StartTrainingResponse},
    },
};
use std::sync::{Arc, Mutex};

pub struct MinSelector;

impl Selector for MinSelector {
    fn select(
        &mut self,
        min_count: usize,
        waiting: impl Iterator<Item = ClientId>,
        _selected: impl Iterator<Item = ClientId>,
    ) -> Vec<ClientId> {
        waiting.take(min_count).collect()
    }
}

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

#[derive(Clone, Default)]
pub struct MutexSelector {
    waiting: Arc<Mutex<Vec<ClientId>>>,
    selected: Arc<Mutex<Vec<ClientId>>>,
    result: Arc<Mutex<Vec<ClientId>>>,
}

// impl MutexSelector {
//     pub fn new() -> Self {
//         Self::default()
//     }
// }

impl Selector for MutexSelector {
    fn select(
        &mut self,
        _min_count: usize,
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

#[derive(Clone)]
pub struct ServiceHandle(InnerServiceHandle);

impl ServiceHandle {
    pub fn new() -> (Self, ServiceRequests) {
        let (inner, requests) = InnerServiceHandle::new();
        (Self(inner), requests)
    }

    pub async fn rendez_vous_accepted(&self) -> ClientId {
        match self.0.rendez_vous().await.unwrap() {
            RendezVousResponse::Accept(id) => id,
            RendezVousResponse::Reject => panic!("rendez-vous rejected"),
        }
    }

    pub async fn heartbeat_selected(&self, id: ClientId) -> u32 {
        match self.0.heartbeat(id).await.unwrap() {
            HeartBeatResponse::Round(round) => round,
            resp => panic!("expected HeartBeatResponse::Round(_) got {:?}", resp),
        }
    }

    pub async fn heartbeat(&self, id: ClientId) -> HeartBeatResponse {
        self.0.heartbeat(id).await.unwrap()
    }

    pub async fn start_training_accepted(&self, id: ClientId) -> (String, Token) {
        match self.0.start_training(id).await.unwrap() {
            StartTrainingResponse::Accept(url, token) => (url, token),
            StartTrainingResponse::Reject => panic!("start_training rejected"),
        }
    }

    pub async fn end_training(&self, id: ClientId, success: bool) {
        self.0.end_training(id, success).await
    }
}
