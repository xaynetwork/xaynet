pub mod builder;
pub mod impls;
pub mod initializer;
pub mod utils;

use serial_test::serial;

use crate::state_machine::{
    events::Event,
    phases::PhaseName,
    tests::{
        builder::StateMachineBuilder,
        utils::{enable_logging, generate_summer, generate_updater, Participant},
    },
};
use xaynet_core::{
    common::{RoundParameters, RoundSeed},
    crypto::{ByteObject, EncryptKeyPair},
    mask::{FromPrimitives, Model},
};

#[tokio::test]
#[serial]
async fn integration_full_round() {
    enable_logging();
    let round_params = RoundParameters {
        pk: EncryptKeyPair::generate().public,
        sum: 0.5,
        update: 1.0,
        seed: RoundSeed::generate(),
    };
    let n_updaters = 3;
    let n_summers = 2;
    let model_size = 4;

    let (state_machine, requests, events, mut eio) = StateMachineBuilder::new()
        .await
        .with_round_id(42)
        .with_seed(round_params.seed.clone())
        .with_sum_ratio(round_params.sum)
        .with_update_ratio(round_params.update)
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
    let summer_1 = generate_summer(round_params.clone());
    let summer_2 = generate_summer(round_params.clone());
    let msg_1 = summer_1.compose_sum_message();
    let msg_2 = summer_2.compose_sum_message();
    let req_1 = async { requests.msg(&msg_1).await.unwrap() };
    let req_2 = async { requests.msg(&msg_2).await.unwrap() };
    let transition = async { state_machine.next().await.unwrap() };
    let ((), (), state_machine) = tokio::join!(req_1, req_2, transition);
    assert!(state_machine.is_update());

    // Update phase
    let transition_task = tokio::spawn(async { state_machine.next().await.unwrap() });
    let sum_dict = events.sum_dict_listener().get_latest().event.unwrap();
    let scalar = 1.0 / (n_updaters as f64 * round_params.update);
    let model = Model::from_primitives(vec![0; model_size].into_iter()).unwrap();
    for _ in 0..3 {
        let updater = generate_updater(round_params.clone());
        let (mask_seed, masked_model) = updater.compute_masked_model(&model, scalar);
        let local_seed_dict = Participant::build_seed_dict(&sum_dict, &mask_seed);
        let msg = updater.compose_update_message(masked_model.clone(), local_seed_dict.clone());
        requests.msg(&msg).await.unwrap();
    }
    let state_machine = transition_task.await.unwrap();
    assert!(state_machine.is_sum2());

    // Sum2 phase
    let seed_dict = events.seed_dict_listener().get_latest().event.unwrap();

    let seeds_1 = summer_1.decrypt_seeds(&seed_dict.get(&summer_1.keys.public).unwrap());
    let aggregation_1 = summer_1.aggregate_masks(model_size, &seeds_1);
    let msg_1 = summer_1.compose_sum2_message(aggregation_1.into());

    let seeds_2 = summer_2.decrypt_seeds(&seed_dict.get(&summer_2.keys.public).unwrap());
    let aggregation_2 = summer_2.aggregate_masks(model_size, &seeds_2);
    let msg_2 = summer_2.compose_sum2_message(aggregation_2.into());

    let req_1 = async { requests.msg(&msg_1).await.unwrap() };
    let req_2 = async { requests.msg(&msg_2).await.unwrap() };

    let transition = async { state_machine.next().await.unwrap() };
    let ((), (), state_machine) = tokio::join!(req_1, req_2, transition);
    assert!(state_machine.is_unmask());

    // Unmask phase
    let state_machine = state_machine.next().await.unwrap();

    // check if a global model exist
    #[cfg(feature = "model-persistence")]
    {
        use crate::storage::{s3, CoordinatorStorage, ModelStorage};

        let round_id = events.params_listener().get_latest().round_id;
        let round_seed = events.params_listener().get_latest().event.seed;
        let global_model_id = s3::Client::create_global_model_id(round_id, &round_seed);

        let s3_model = eio
            .s3
            .global_model(&global_model_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            matches!(events.model_listener().get_latest().event, super::events::ModelUpdate::New(broadcasted_model) if s3_model == *broadcasted_model)
        );

        let get_global_model_id = eio.redis.latest_global_model_id().await.unwrap().unwrap();
        assert_eq!(global_model_id, get_global_model_id);
    }

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

    // check if all seed dicts have been removed
    for (sum_pk, _) in sum_dict.iter() {
        assert!(eio
            .redis
            .seed_dict_for_sum_pk(sum_pk)
            .await
            .unwrap()
            .is_empty());
    }

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
