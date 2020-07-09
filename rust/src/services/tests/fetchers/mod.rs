use tokio_test::assert_ready;
use tower_test::mock::Spawn;

use crate::{
    services::{
        fetchers::{MaskLengthRequest, MaskLengthService},
        tests::utils::new_event_channels,
    },
    state_machine::events::MaskLengthUpdate,
};

#[tokio::test]
async fn test_mask_length_svc() {
    let (mut publisher, subscriber) = new_event_channels();

    let mut task = Spawn::new(MaskLengthService::new(&subscriber));
    assert_ready!(task.poll_ready()).unwrap();

    let resp = task.call(MaskLengthRequest).await;
    assert_eq!(resp, Ok(None));

    let round_id = subscriber.params_listener().get_latest().round_id;
    publisher.broadcast_mask_length(round_id.clone(), MaskLengthUpdate::New(42));
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(MaskLengthRequest).await;
    assert_eq!(resp, Ok(Some(42)));

    publisher.broadcast_mask_length(round_id, MaskLengthUpdate::Invalidate);
    assert_ready!(task.poll_ready()).unwrap();
    let resp = task.call(MaskLengthRequest).await;
    assert_eq!(resp, Ok(None));
}
