use std::sync::Arc;

use rayon::ThreadPoolBuilder;
use tokio_test::assert_ready;
use tower_test::mock::Spawn;

use crate::{
    crypto::{ByteObject, PublicEncryptKey, SigningKeyPair},
    message::{MessageOwned, MessageSeal, SumOwned},
    services::{
        messages::{
            MessageParserError,
            MessageParserRequest,
            MessageParserResponse,
            MessageParserService,
        },
        tests::utils::new_event_channels,
    },
    state_machine::{
        coordinator::RoundParameters,
        events::{EventPublisher, EventSubscriber, PhaseEvent},
    },
    utils::trace::Traced,
};

fn spawn_svc() -> (EventPublisher, EventSubscriber, Spawn<MessageParserService>) {
    let (publisher, subscriber) = new_event_channels();
    let thread_pool = Arc::new(ThreadPoolBuilder::new().build().unwrap());
    let task = Spawn::new(MessageParserService::new(&subscriber, thread_pool));
    (publisher, subscriber, task)
}

fn make_req(bytes: Vec<u8>) -> Traced<MessageParserRequest> {
    Traced::new(bytes.into(), error_span!("test"))
}

fn new_sum_message(round_params: &RoundParameters) -> (MessageOwned, Vec<u8>) {
    // Simulate a sum participant crypto material
    let participant_ephm_pk = PublicEncryptKey::generate();
    let participant_signing_keys = SigningKeyPair::generate();

    // Create the message payload
    let sum_signature = participant_signing_keys
        .secret
        .sign_detached(&[round_params.seed.as_slice(), b"sum"].concat());
    let payload = SumOwned {
        sum_signature,
        ephm_pk: participant_ephm_pk,
    };

    // Create the message itself
    let message = MessageOwned::new_sum(
        round_params.pk.clone(),
        participant_signing_keys.public.clone(),
        payload,
    );

    // Encrypt the message
    let seal = MessageSeal {
        recipient_pk: &round_params.pk,
        sender_sk: &participant_signing_keys.secret,
    };
    let encrypted_message = seal.seal(&message);

    (message, encrypted_message)
}

#[tokio::test]
async fn test_decrypt_fail() {
    let (_publisher, _subscriber, mut task) = spawn_svc();
    assert_ready!(task.poll_ready()).unwrap();

    let req = make_req(vec![0, 1, 2, 3, 4, 5, 6]);
    let resp: Result<MessageParserResponse, ::std::convert::Infallible> = task.call(req).await;
    // this is a bit weird because MessageParserError doesn't impl Eq
    // and PartialEq
    match resp {
        Ok(Err(MessageParserError::Decrypt)) => {}
        _ => panic!("expected decrypt error"),
    }
    assert_ready!(task.poll_ready()).unwrap();
}

#[tokio::test]
async fn test_valid_request() {
    let (mut publisher, subscriber, mut task) = spawn_svc();
    assert_ready!(task.poll_ready()).unwrap();

    let round_params = subscriber.params_listener().get_latest().event;
    let (message, encrypted_message) = new_sum_message(&round_params);
    let req = make_req(encrypted_message);

    // Simulate the state machine broadcasting the sum phase
    // (otherwise the request will be rejected)
    publisher.broadcast_phase(round_params.seed.clone(), PhaseEvent::Sum);

    // Call the service
    let resp = task.call(req).await.unwrap().unwrap();
    assert_eq!(resp, message);
}

#[tokio::test]
async fn test_unexpected_message() {
    let (_publisher, subscriber, mut task) = spawn_svc();
    assert_ready!(task.poll_ready()).unwrap();

    let round_params = subscriber.params_listener().get_latest().event;
    let (_, encrypted_message) = new_sum_message(&round_params);
    let req = make_req(encrypted_message);

    let err = task.call(req).await.unwrap().unwrap_err();
    match err {
        MessageParserError::UnexpectedMessage => {}
        _ => panic!(
            "expected MessageParserError::UnexpectedMessage got {:?}",
            err
        ),
    }
}
