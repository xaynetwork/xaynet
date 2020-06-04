use std::{pin::Pin, sync::Arc, task::Poll};

use futures::{future::Future, task::Context};
use tokio::sync::Mutex;
use tower::Service;

use crate::{
    coordinator::Coordinator,
    services::error::{RequestFailed, ServiceError},
};

use crate::{
    message::{MessageOwned, PayloadOwned},
    ParticipantPublicKey,
    SumParticipantEphemeralPublicKey,
};

// TODO: add some ID so that the service that holds all the data can
// invalidate the request if the state changed while the request was
// going through all the layers. This is some kind of optimistic
// locking.
#[derive(Debug)]
pub struct SumRequest {
    pub participant_pk: ParticipantPublicKey,
    pub ephm_pk: SumParticipantEphemeralPublicKey,
}
#[derive(Debug)]
pub enum StateMachineRequest {
    Sum(SumRequest),
    /* Update(UpdateRequest),
     * Sum2(Sum2Request), */
}

impl From<MessageOwned> for StateMachineRequest {
    fn from(message: MessageOwned) -> Self {
        let MessageOwned { header, payload } = message;
        match payload {
            PayloadOwned::Sum(sum_payload) => {
                let sum_req = SumRequest {
                    participant_pk: header.participant_pk,
                    ephm_pk: sum_payload.ephm_pk,
                };
                StateMachineRequest::Sum(sum_req)
            }
            _ => unimplemented!(),
        }
    }
}
pub type StateMachineResponse = Result<(), RequestFailed>;

pub struct StateMachineService {
    coordinator: Arc<Mutex<Coordinator>>,
}

impl StateMachineService {
    pub fn new(coordinator: Coordinator) -> Self {
        Self {
            coordinator: Arc::new(Mutex::new(coordinator)),
        }
    }
}

impl Service<StateMachineRequest> for StateMachineService {
    type Response = StateMachineResponse;
    type Error = ServiceError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + 'static + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: StateMachineRequest) -> Self::Future {
        let mutex = self.coordinator.clone();
        let fut = async move {
            let mut coordinator = mutex.lock_owned().await;
            match req {
                StateMachineRequest::Sum(sum_req) => {
                    let SumRequest {
                        participant_pk,
                        ephm_pk,
                    } = sum_req;
                    Ok(coordinator
                        .handle_sum(participant_pk, ephm_pk)
                        .map_err(|_e| RequestFailed::Other))
                }
            }
        };
        Box::pin(fut)
    }
}
