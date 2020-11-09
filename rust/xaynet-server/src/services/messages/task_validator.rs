use std::task::Poll;

use futures::{future, task::Context};
use tower::Service;

use crate::{
    services::messages::ServiceError,
    state_machine::events::{EventListener, EventSubscriber},
};
use xaynet_core::{
    common::RoundParameters,
    crypto::ByteObject,
    message::{Message, Payload},
};

/// A service for performing sanity checks and preparing incoming
/// requests to be handled by the state machine.
#[derive(Clone, Debug)]
pub struct TaskValidator {
    params_listener: EventListener<RoundParameters>,
}

impl TaskValidator {
    pub fn new(subscriber: &EventSubscriber) -> Self {
        Self {
            params_listener: subscriber.params_listener(),
        }
    }
}

impl Service<Message> for TaskValidator {
    type Response = Message;
    type Error = ServiceError;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, message: Message) -> Self::Future {
        let (sum_signature, update_signature) = match message.payload {
            Payload::Sum(ref sum) => (sum.sum_signature, None),
            Payload::Update(ref update) => (update.sum_signature, Some(update.update_signature)),
            Payload::Sum2(ref sum2) => (sum2.sum_signature, None),
            _ => return future::ready(Err(ServiceError::UnexpectedMessage)),
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
                    future::ready(Ok(message))
                } else {
                    future::ready(Err(ServiceError::NotSumEligible))
                }
            }
            Payload::Update(_) => {
                if is_updater {
                    future::ready(Ok(message))
                } else {
                    future::ready(Err(ServiceError::NotUpdateEligible))
                }
            }
            _ => future::ready(Err(ServiceError::UnexpectedMessage)),
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio_test::assert_ready;
    use tower_test::mock::Spawn;

    use crate::{
        services::tests::utils,
        state_machine::{
            events::{EventPublisher, EventSubscriber},
            phases::PhaseName,
        },
    };

    use super::*;

    fn spawn_svc() -> (EventPublisher, EventSubscriber, Spawn<TaskValidator>) {
        let (publisher, subscriber) = utils::new_event_channels();
        let task = Spawn::new(TaskValidator::new(&subscriber));
        (publisher, subscriber, task)
    }

    #[tokio::test]
    async fn test_sum_ok() {
        let (mut publisher, subscriber, mut task) = spawn_svc();

        let mut round_params = subscriber.params_listener().get_latest().event;

        // make sure everyone is eligible
        round_params.sum = 1.0;

        publisher.broadcast_params(round_params.clone());
        publisher.broadcast_phase(PhaseName::Sum);

        let (message, _) = utils::new_sum_message(&round_params);

        assert_ready!(task.poll_ready()).unwrap();
        let resp = task.call(message.clone()).await.unwrap();
        assert_eq!(resp, message);
    }

    #[tokio::test]
    async fn test_sum_not_eligible() {
        let (mut publisher, subscriber, mut task) = spawn_svc();

        let mut round_params = subscriber.params_listener().get_latest().event;

        // make sure no-one is eligible
        round_params.sum = 0.0;

        publisher.broadcast_params(round_params.clone());
        publisher.broadcast_phase(PhaseName::Sum);

        let (message, _) = utils::new_sum_message(&round_params);

        assert_ready!(task.poll_ready()).unwrap();
        let err = task.call(message).await.unwrap_err();
        match err {
            ServiceError::NotSumEligible => {}
            _ => panic!("expected ServiceError::NotSumEligible got {:?}", err),
        }
    }
}
