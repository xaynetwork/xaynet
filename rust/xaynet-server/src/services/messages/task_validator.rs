use std::task::Poll;

use futures::{future, task::Context};
use thiserror::Error;
use tower::Service;
use xaynet_core::{
    common::RoundParameters,
    crypto::ByteObject,
    message::{Message, Payload},
};

use crate::{
    state_machine::events::{EventListener, EventSubscriber},
    utils::request::Request,
};

/// A service for performing sanity checks and preparing incoming
/// requests to be handled by the state machine.
pub struct TaskValidatorService {
    params_listener: EventListener<RoundParameters>,
}

impl TaskValidatorService {
    pub fn new(subscriber: &EventSubscriber) -> Self {
        Self {
            params_listener: subscriber.params_listener(),
        }
    }
}

/// Request type for [`TaskValidatorService`]
pub type TaskValidatorRequest = Request<Message>;

/// Response type for [`TaskValidatorService`]
pub type TaskValidatorResponse = Result<Message, TaskValidatorError>;

impl Service<TaskValidatorRequest> for TaskValidatorService {
    type Response = TaskValidatorResponse;
    type Error = std::convert::Infallible;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: TaskValidatorRequest) -> Self::Future {
        let message = req.into_inner();
        let (sum_signature, update_signature) = match message.payload {
            Payload::Sum(ref sum) => (sum.sum_signature, None),
            Payload::Update(ref update) => (update.sum_signature, Some(update.update_signature)),
            Payload::Sum2(ref sum2) => (sum2.sum_signature, None),
            _ => return future::ready(Ok(Err(TaskValidatorError::UnexpectedMessage))),
        };
        let params = self.params_listener.get_latest().event;
        let seed = params.seed.as_slice();

        // Check whether the participant is eligible for the sum task
        let has_valid_sum_signature = message
            .participant_pk
            .verify_detached(&sum_signature, &[seed, b"sum"].concat());
        let is_summer = has_valid_sum_signature && sum_signature.is_eligible(params.sum);

        // Check whether the participant is eligible for the update task
        let has_valid_update_signature = update_signature
            .map(|sig| {
                message
                    .participant_pk
                    .verify_detached(&sig, &[seed, b"update"].concat())
            })
            .unwrap_or(false);
        let is_updater = !is_summer
            && has_valid_update_signature
            && update_signature
                .map(|sig| sig.is_eligible(params.update))
                .unwrap_or(false);

        match message.payload {
            Payload::Sum(_) | Payload::Sum2(_) => {
                if is_summer {
                    future::ready(Ok(Ok(message)))
                } else {
                    future::ready(Ok(Err(TaskValidatorError::NotSumEligible)))
                }
            }
            Payload::Update(_) => {
                if is_updater {
                    future::ready(Ok(Ok(message)))
                } else {
                    future::ready(Ok(Err(TaskValidatorError::NotUpdateEligible)))
                }
            }
            _ => future::ready(Ok(Err(TaskValidatorError::UnexpectedMessage))),
        }
    }
}

/// Error type for [`TaskValidatorService`]
#[derive(Error, Debug)]
pub enum TaskValidatorError {
    #[error("Not eligible for sum task")]
    NotSumEligible,

    #[error("Not eligible for update task")]
    NotUpdateEligible,

    #[error("The message was rejected because the coordinator did not expect it")]
    UnexpectedMessage,

    #[error("Internal error")]
    InternalError,
}
