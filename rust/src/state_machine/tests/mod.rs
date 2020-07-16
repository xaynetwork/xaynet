pub mod builder;
pub mod impls;
pub mod utils;

use crate::state_machine::tests::{
    builder::StateMachineBuilder,
    utils::{enable_logging, gen_sum2_request, gen_sum_request, gen_update_request},
};

#[tokio::test]
async fn full_round() {
    enable_logging();

    let (mut state_machine, request_tx, _events_subscriber) = StateMachineBuilder::new().build();
    assert!(state_machine.is_idle());

    state_machine = state_machine.next().await.unwrap(); // transition from init to sum state
    assert!(state_machine.is_sum());

    let (sum_req, sum_pk, response_rx) = gen_sum_request();
    let _ = request_tx.send(sum_req);

    state_machine = state_machine.next().await.unwrap(); // transition from sum to update state
    assert!(state_machine.is_update());
    assert!(response_rx.await.is_ok());

    for _ in 0..3 {
        let (req, _) = gen_update_request(sum_pk.clone());
        let _ = request_tx.send(req);
    }
    state_machine = state_machine.next().await.unwrap(); // transition from update to sum state
    assert!(state_machine.is_sum2());

    let (req, response_rx) = gen_sum2_request(sum_pk.clone());
    let _ = request_tx.send(req);
    state_machine = state_machine.next().await.unwrap(); // transition from sum2 to unmasked state
    assert!(response_rx.await.is_ok());
    assert!(state_machine.is_unmask());

    state_machine = state_machine.next().await.unwrap(); // transition from unmasked to idle state
    assert!(state_machine.is_idle());

    drop(request_tx);
    state_machine = state_machine.next().await.unwrap(); // transition from idle to sum state
    assert!(state_machine.is_sum());

    state_machine = state_machine.next().await.unwrap(); // transition from sum to error state
    assert!(state_machine.is_error());

    state_machine = state_machine.next().await.unwrap(); // transition from error to shutdown state
    assert!(state_machine.is_shutdown());
    assert!(state_machine.next().await.is_none())
}
