pub mod builder;
pub mod impls;
pub mod utils;

use xaynet_core::{
    common::RoundSeed,
    crypto::{ByteObject, EncryptKeyPair},
    mask::{FromPrimitives, Model},
};

use crate::state_machine::{
    events::Event,
    phases::PhaseName,
    tests::{
        builder::StateMachineBuilder,
        utils::{enable_logging, generate_summer, generate_updater},
    },
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn integration_full_round() {
    enable_logging();
    let n_updaters = 3;
    let n_summers = 2;
    let seed = RoundSeed::generate();
    let sum_ratio = 0.5;
    let update_ratio = 1.0;
    let coord_keys = EncryptKeyPair::generate();
    let coord_pk = coord_keys.public;
    let model_size = 4;

    let (state_machine, requests, events, _) = StateMachineBuilder::new()
        .await
        .with_round_id(42)
        .with_seed(seed.clone())
        .with_sum_ratio(sum_ratio)
        .with_update_ratio(update_ratio)
        .with_min_sum(n_summers)
        .with_min_update(n_updaters)
        .with_min_sum_time(1)
        .with_max_sum_time(2)
        .with_min_update_time(1)
        .with_max_update_time(2)
        .with_model_size(model_size)
        .build();

    assert!(state_machine.is_idle());

    // Idle phase
    let state_machine = state_machine.next().await.unwrap();
    assert!(state_machine.is_sum());

    // Sum phase
    let mut summer_1 = generate_summer(&seed, sum_ratio, update_ratio);
    let mut summer_2 = generate_summer(&seed, sum_ratio, update_ratio);
    let msg_1 = summer_1.compose_sum_message(coord_pk);
    let msg_2 = summer_2.compose_sum_message(coord_pk);
    let req_1 = async { requests.msg(&msg_1).await.unwrap() };
    let req_2 = async { requests.msg(&msg_2).await.unwrap() };
    let transition = async { state_machine.next().await.unwrap() };
    let ((), (), state_machine) = tokio::join!(req_1, req_2, transition);
    assert!(state_machine.is_update());

    // Update phase
    let transition_task = tokio::spawn(async { state_machine.next().await.unwrap() });
    let sum_dict = events.sum_dict_listener().get_latest().event.unwrap();
    let scalar = 1.0 / (n_updaters as f64 * update_ratio);
    let model = Model::from_primitives(vec![0; model_size].into_iter()).unwrap();
    for _ in 0..3 {
        let updater = generate_updater(&seed, sum_ratio, update_ratio);
        let msg = updater.compose_update_message(coord_pk, &sum_dict, scalar, model.clone());
        requests.msg(&msg).await.unwrap();
    }
    let state_machine = transition_task.await.unwrap();
    assert!(state_machine.is_sum2());

    // Sum2 phase
    let seed_dict = events.seed_dict_listener().get_latest().event.unwrap();
    let mask_length = events.mask_length_listener().get_latest().event.unwrap();
    let msg_1 = summer_1
        .compose_sum2_message(coord_pk, seed_dict.get(&summer_1.pk).unwrap(), mask_length)
        .unwrap();
    let msg_2 = summer_2
        .compose_sum2_message(coord_pk, seed_dict.get(&summer_2.pk).unwrap(), mask_length)
        .unwrap();
    let req_1 = async { requests.msg(&msg_1).await.unwrap() };
    let req_2 = async { requests.msg(&msg_2).await.unwrap() };
    let transition = async { state_machine.next().await.unwrap() };
    let ((), (), state_machine) = tokio::join!(req_1, req_2, transition);
    assert!(state_machine.is_unmask());

    // Unmask phase
    let state_machine = state_machine.next().await.unwrap();
    assert!(state_machine.is_idle());

    // New idle phase
    let state_machine = state_machine.next().await.unwrap();
    // During the idle phase, a new phase event with an updated round
    // id should have been emitted.
    assert_eq!(
        Event {
            round_id: 43,
            event: PhaseName::Idle,
        },
        events.phase_listener().get_latest()
    );

    // dropping the request sender should make the state machine
    // error out
    drop(requests);
    let state_machine = state_machine.next().await.unwrap();
    assert!(state_machine.is_error());

    // then the state machine should enter the shutdown state
    let state_machine = state_machine.next().await.unwrap();
    assert!(state_machine.is_shutdown());
    assert!(state_machine.next().await.is_none())
}
