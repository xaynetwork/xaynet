use tokio_test::assert_ready;
use tower_test::mock::Spawn;

use crate::{
    message::MessageOwned,
    services::{
        messages::{PreProcessorError, PreProcessorRequest, PreProcessorService},
        tests::utils,
    },
    state_machine::{
        events::{EventPublisher, EventSubscriber},
        phases::PhaseName,
    },
    utils::trace::Traced,
};

fn spawn_svc() -> (EventPublisher, EventSubscriber, Spawn<PreProcessorService>) {
    let (publisher, subscriber) = utils::new_event_channels();
    let task = Spawn::new(PreProcessorService::new(&subscriber));
    (publisher, subscriber, task)
}

fn make_req(message: MessageOwned) -> Traced<PreProcessorRequest> {
    Traced::new(message.into(), error_span!("test"))
}

#[tokio::test]
async fn test_sum_ok() {
    let (mut publisher, subscriber, mut task) = spawn_svc();

    let mut round_params = subscriber.params_listener().get_latest().event;

    // make sure everyone is eligible
    round_params.sum = 1.0;

    let round_id = round_params.seed.clone();
    publisher.broadcast_params(round_params.clone());
    publisher.broadcast_phase(round_id, PhaseName::Sum);

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

    let round_id = round_params.seed.clone();
    publisher.broadcast_params(round_params.clone());
    publisher.broadcast_phase(round_id, PhaseName::Sum);

    let (message, _, _) = utils::new_sum_message(&round_params);
    let req = make_req(message.clone());

    assert_ready!(task.poll_ready()).unwrap();
    let err = task.call(req).await.unwrap().unwrap_err();
    match err {
        PreProcessorError::NotSumEligible => {}
        _ => panic!("expected PreProcessorError::NotSumEligible got {:?}", err),
    }
}

// This is a corner case which should almost never happen but is worth
// testing: in `poll_ready`, the service checks the current phase, and
// calls `poll_ready` on the appropriate service based on that. Then
// the request is processed by `call` but if the phase has changed in
// the meantime, we want to reject the request, because the service
// that should process it is not the one on which we called
// `poll_ready` previously.
#[tokio::test]
async fn test_phase_change_between_poll_ready_and_call() {
    let (mut publisher, subscriber, mut task) = spawn_svc();
    // call poll_ready here
    assert_ready!(task.poll_ready()).unwrap();

    let round_params = subscriber.params_listener().get_latest().event;
    let (message, _, _) = utils::new_sum_message(&round_params);
    let req = make_req(message.clone());

    publisher.broadcast_phase(round_params.seed.clone(), PhaseName::Sum);

    let err = task.call(req).await.unwrap().unwrap_err();
    match err {
        PreProcessorError::UnexpectedMessage => {}
        _ => panic!(
            "expected PreProcessorError::UnexpectedMessage got {:?}",
            err
        ),
    }
}
