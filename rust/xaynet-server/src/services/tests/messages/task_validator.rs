use tokio_test::assert_ready;
use tower_test::mock::Spawn;
use xaynet_core::message::Message;

use crate::{
    services::{
        messages::{TaskValidatorError, TaskValidatorRequest, TaskValidatorService},
        tests::utils,
    },
    state_machine::{
        events::{EventPublisher, EventSubscriber},
        phases::PhaseName,
    },
    utils::Request,
};

fn spawn_svc() -> (EventPublisher, EventSubscriber, Spawn<TaskValidatorService>) {
    let (publisher, subscriber) = utils::new_event_channels();
    let task = Spawn::new(TaskValidatorService::new(&subscriber));
    (publisher, subscriber, task)
}

fn make_req(message: Message) -> TaskValidatorRequest {
    Request::new(message)
}

#[tokio::test]
async fn test_sum_ok() {
    let (mut publisher, subscriber, mut task) = spawn_svc();

    let mut round_params = subscriber.params_listener().get_latest().event;

    // make sure everyone is eligible
    round_params.sum = 1.0;

    publisher.broadcast_params(round_params.clone());
    publisher.broadcast_phase(PhaseName::Sum);

    let (message, _, _) = utils::new_sum_message(&round_params);
    let req = make_req(message.clone());

    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(req).await.unwrap().unwrap();
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

    let (message, _, _) = utils::new_sum_message(&round_params);
    let req = make_req(message.clone());

    assert_ready!(task.poll_ready()).unwrap();
    let err = task.call(req).await.unwrap().unwrap_err();
    match err {
        TaskValidatorError::NotSumEligible => {}
        _ => panic!("expected TaskValidatorError::NotSumEligible got {:?}", err),
    }
}
