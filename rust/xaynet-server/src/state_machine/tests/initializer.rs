use serial_test::serial;

use super::utils::{mask_settings, model_settings, pet_settings};
#[cfg(feature = "metrics")]
use crate::metrics::MetricsSender;
#[cfg(feature = "model-persistence")]
use crate::{
    settings::RestoreSettings,
    state_machine::{
        events::{DictionaryUpdate, MaskLengthUpdate, ModelUpdate},
        phases::PhaseName,
        StateMachineInitializationError,
    },
    storage::ModelStorage,
    storage::{s3, tests::create_global_model},
};
use crate::{
    state_machine::{CoordinatorState, StateMachineInitializer},
    storage::{api::CoordinatorStorage, redis},
};

#[cfg(feature = "model-persistence")]
#[tokio::test]
#[serial]
async fn integration_state_machine_initializer_no_restore() {
    let redis = redis::tests::init_client().await;
    let smi = StateMachineInitializer::new(
        pet_settings(),
        mask_settings(),
        model_settings(),
        RestoreSettings { enable: false },
        redis,
        s3::tests::create_client().await,
        #[cfg(feature = "metrics")]
        MetricsSender(),
    );

    let (state_machine, _request_sender, event_subscriber) = smi.init().await.unwrap();

    assert!(state_machine.is_idle());

    let phase = event_subscriber.phase_listener().get_latest().event;
    assert!(matches!(phase, PhaseName::Idle));

    let sum_dict = event_subscriber.sum_dict_listener().get_latest().event;
    assert!(matches!(sum_dict, DictionaryUpdate::Invalidate));

    let seed_dict = event_subscriber.seed_dict_listener().get_latest().event;
    assert!(matches!(seed_dict, DictionaryUpdate::Invalidate));

    let mask_length = event_subscriber.mask_length_listener().get_latest().event;
    assert!(matches!(mask_length, MaskLengthUpdate::Invalidate));

    let global_model = event_subscriber.model_listener().get_latest().event;
    assert!(matches!(global_model, ModelUpdate::Invalidate));

    let round_id = event_subscriber.params_listener().get_latest().round_id;
    assert_eq!(round_id, 0);
}

#[cfg(feature = "model-persistence")]
#[tokio::test]
#[serial]
async fn integration_state_machine_initializer_no_state() {
    let redis = redis::tests::init_client().await;
    let smi = StateMachineInitializer::new(
        pet_settings(),
        mask_settings(),
        model_settings(),
        RestoreSettings { enable: true },
        redis,
        s3::tests::create_client().await,
        #[cfg(feature = "metrics")]
        MetricsSender(),
    );

    let (state_machine, _request_sender, event_subscriber) = smi.init().await.unwrap();

    assert!(state_machine.is_idle());

    let phase = event_subscriber.phase_listener().get_latest().event;
    assert!(matches!(phase, PhaseName::Idle));

    let sum_dict = event_subscriber.sum_dict_listener().get_latest().event;
    assert!(matches!(sum_dict, DictionaryUpdate::Invalidate));

    let seed_dict = event_subscriber.seed_dict_listener().get_latest().event;
    assert!(matches!(seed_dict, DictionaryUpdate::Invalidate));

    let mask_length = event_subscriber.mask_length_listener().get_latest().event;
    assert!(matches!(mask_length, MaskLengthUpdate::Invalidate));

    let global_model = event_subscriber.model_listener().get_latest().event;
    assert!(matches!(global_model, ModelUpdate::Invalidate));

    let round_id = event_subscriber.params_listener().get_latest().round_id;
    assert_eq!(round_id, 0);
}

#[cfg(feature = "model-persistence")]
#[tokio::test]
#[serial]
async fn integration_state_machine_initializer_without_global_model() {
    let pet_settings = pet_settings();
    let mask_settings = mask_settings();
    let model_settings = model_settings();

    // set a coordinator state in redis with the round_id 5
    let mut redis = redis::tests::init_client().await;
    let mut state = CoordinatorState::new(pet_settings, mask_settings, model_settings.clone());
    let new_round_id = 5;
    state.round_id = new_round_id;
    redis.set_coordinator_state(&state).await.unwrap();

    let smi = StateMachineInitializer::new(
        pet_settings,
        mask_settings,
        model_settings,
        RestoreSettings { enable: true },
        redis,
        s3::tests::create_client().await,
        #[cfg(feature = "metrics")]
        MetricsSender(),
    );

    let (state_machine, _request_sender, event_subscriber) = smi.init().await.unwrap();

    assert!(state_machine.is_idle());

    let phase = event_subscriber.phase_listener().get_latest().event;
    assert!(matches!(phase, PhaseName::Idle));

    let sum_dict = event_subscriber.sum_dict_listener().get_latest().event;
    assert!(matches!(sum_dict, DictionaryUpdate::Invalidate));

    let seed_dict = event_subscriber.seed_dict_listener().get_latest().event;
    assert!(matches!(seed_dict, DictionaryUpdate::Invalidate));

    let mask_length = event_subscriber.mask_length_listener().get_latest().event;
    assert!(matches!(mask_length, MaskLengthUpdate::Invalidate));

    let global_model = event_subscriber.model_listener().get_latest().event;
    assert!(matches!(global_model, ModelUpdate::Invalidate));

    let round_id = event_subscriber.params_listener().get_latest().round_id;
    assert_eq!(round_id, new_round_id);
}

#[cfg(feature = "model-persistence")]
#[tokio::test]
#[serial]
async fn integration_state_machine_initializer_with_global_model() {
    let pet_settings = pet_settings();
    let mask_settings = mask_settings();
    let model_settings = model_settings();

    // set a coordinator state in redis with the round_id 7
    let mut redis = redis::tests::init_client().await;
    let mut state = CoordinatorState::new(pet_settings, mask_settings, model_settings.clone());
    let new_round_id = 7;
    state.round_id = new_round_id;
    redis.set_coordinator_state(&state).await.unwrap();

    // upload a global model to minio and set the id in redis
    let uploaded_global_model = create_global_model(state.model_size);
    let mut s3 = s3::tests::create_client().await;
    let global_model_id = s3
        .set_global_model(
            state.round_id,
            &state.round_params.seed,
            &uploaded_global_model,
        )
        .await
        .unwrap();
    redis
        .set_latest_global_model_id(&global_model_id)
        .await
        .unwrap();

    let smi = StateMachineInitializer::new(
        pet_settings,
        mask_settings,
        model_settings,
        RestoreSettings { enable: true },
        redis,
        s3,
        #[cfg(feature = "metrics")]
        MetricsSender(),
    );

    let (state_machine, _request_sender, event_subscriber) = smi.init().await.unwrap();

    assert!(state_machine.is_idle());

    let phase = event_subscriber.phase_listener().get_latest().event;
    assert!(matches!(phase, PhaseName::Idle));

    let sum_dict = event_subscriber.sum_dict_listener().get_latest().event;
    assert!(matches!(sum_dict, DictionaryUpdate::Invalidate));

    let seed_dict = event_subscriber.seed_dict_listener().get_latest().event;
    assert!(matches!(seed_dict, DictionaryUpdate::Invalidate));

    let mask_length = event_subscriber.mask_length_listener().get_latest().event;
    assert!(matches!(mask_length, MaskLengthUpdate::Invalidate));

    let global_model = event_subscriber.model_listener().get_latest().event;
    assert!(
        matches!(global_model, ModelUpdate::New(broadcasted_model) if uploaded_global_model == *broadcasted_model)
    );

    let round_id = event_subscriber.params_listener().get_latest().round_id;
    assert_eq!(round_id, new_round_id);
}

#[cfg(feature = "model-persistence")]
#[tokio::test]
#[serial]
async fn integration_state_machine_initializer_failed_because_of_wrong_size() {
    let pet_settings = pet_settings();
    let mask_settings = mask_settings();
    let model_settings = model_settings();

    // set a coordinator state in redis with the round_id 9
    let mut redis = redis::tests::init_client().await;
    let mut state = CoordinatorState::new(pet_settings, mask_settings, model_settings.clone());
    let new_round_id = 9;
    state.round_id = new_round_id;
    redis.set_coordinator_state(&state).await.unwrap();

    // upload a global model with a wrong model size to minio and set the id in redis
    let uploaded_global_model = create_global_model(state.model_size + 10);
    let mut s3 = s3::tests::create_client().await;
    let global_model_id = s3
        .set_global_model(
            state.round_id,
            &state.round_params.seed,
            &uploaded_global_model,
        )
        .await
        .unwrap();
    redis
        .set_latest_global_model_id(&global_model_id)
        .await
        .unwrap();

    let smi = StateMachineInitializer::new(
        pet_settings,
        mask_settings,
        model_settings,
        RestoreSettings { enable: true },
        redis,
        s3,
        #[cfg(feature = "metrics")]
        MetricsSender(),
    );

    let result = smi.init().await;

    assert!(matches!(
        result,
        Err(StateMachineInitializationError::GlobalModelInvalid(_))
    ));
}

#[cfg(feature = "model-persistence")]
#[tokio::test]
#[serial]
async fn integration_state_machine_initializer_failed_to_find_global_model() {
    let pet_settings = pet_settings();
    let mask_settings = mask_settings();
    let model_settings = model_settings();

    // set a coordinator state in redis with the round_id 11
    let mut redis = redis::tests::init_client().await;
    let mut state = CoordinatorState::new(pet_settings, mask_settings, model_settings.clone());
    let new_round_id = 11;
    state.round_id = new_round_id;
    redis.set_coordinator_state(&state).await.unwrap();

    // set a model id in redis but don't upload a model to minio
    let global_model_id =
        s3::Client::create_global_model_id(state.round_id, &state.round_params.seed);
    redis
        .set_latest_global_model_id(&global_model_id)
        .await
        .unwrap();

    let smi = StateMachineInitializer::new(
        pet_settings,
        mask_settings,
        model_settings,
        RestoreSettings { enable: true },
        redis,
        s3::tests::create_client().await,
        #[cfg(feature = "metrics")]
        MetricsSender(),
    );

    let result = smi.init().await;

    assert!(matches!(
        result,
        Err(StateMachineInitializationError::GlobalModelUnavailable(_))
    ));
}

#[tokio::test]
#[serial]
async fn integration_state_machine_initializer_reset_state() {
    let pet_settings = pet_settings();
    let mask_settings = mask_settings();
    let model_settings = model_settings();

    let mut redis = redis::tests::init_client().await;
    let state = CoordinatorState::new(pet_settings, mask_settings, model_settings.clone());
    redis.set_coordinator_state(&state).await.unwrap();

    let mut smi = StateMachineInitializer::new(
        pet_settings,
        mask_settings,
        model_settings,
        #[cfg(feature = "model-persistence")]
        RestoreSettings { enable: true },
        redis.clone(),
        #[cfg(feature = "model-persistence")]
        s3::tests::create_client().await,
        #[cfg(feature = "metrics")]
        MetricsSender(),
    );

    smi.from_settings().await.unwrap();

    let keys = redis.keys().await.unwrap();

    assert!(keys.is_empty());
}
