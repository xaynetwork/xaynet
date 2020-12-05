use serial_test::serial;

use super::utils::{mask_settings, model_settings, pet_settings};
#[cfg(feature = "model-persistence")]
use crate::{
    settings::RestoreSettings,
    state_machine::{
        events::{DictionaryUpdate, ModelUpdate},
        initializer::StateMachineInitializationError,
        phases::PhaseName,
    },
    storage::tests::utils::create_global_model,
    storage::ModelStorage,
};
use crate::{
    state_machine::{coordinator::CoordinatorState, StateMachineInitializer},
    storage::{tests::init_store, CoordinatorStorage},
};

#[cfg(feature = "model-persistence")]
#[tokio::test]
#[serial]
async fn integration_state_machine_initializer_no_restore() {
    let store = init_store().await;
    let smi = StateMachineInitializer::new(
        pet_settings(),
        mask_settings(),
        model_settings(),
        RestoreSettings { enable: false },
        store,
    );

    let (state_machine, _request_sender, event_subscriber) = smi.init().await.unwrap();

    assert!(state_machine.is_idle());

    let phase = event_subscriber.phase_listener().get_latest().event;
    assert!(matches!(phase, PhaseName::Idle));

    let sum_dict = event_subscriber.sum_dict_listener().get_latest().event;
    assert!(matches!(sum_dict, DictionaryUpdate::Invalidate));

    let seed_dict = event_subscriber.seed_dict_listener().get_latest().event;
    assert!(matches!(seed_dict, DictionaryUpdate::Invalidate));

    let global_model = event_subscriber.model_listener().get_latest().event;
    assert!(matches!(global_model, ModelUpdate::Invalidate));

    let round_id = event_subscriber.params_listener().get_latest().round_id;
    assert_eq!(round_id, 0);
}

#[cfg(feature = "model-persistence")]
#[tokio::test]
#[serial]
async fn integration_state_machine_initializer_no_state() {
    let store = init_store().await;
    let smi = StateMachineInitializer::new(
        pet_settings(),
        mask_settings(),
        model_settings(),
        RestoreSettings { enable: true },
        store,
    );

    let (state_machine, _request_sender, event_subscriber) = smi.init().await.unwrap();

    assert!(state_machine.is_idle());

    let phase = event_subscriber.phase_listener().get_latest().event;
    assert!(matches!(phase, PhaseName::Idle));

    let sum_dict = event_subscriber.sum_dict_listener().get_latest().event;
    assert!(matches!(sum_dict, DictionaryUpdate::Invalidate));

    let seed_dict = event_subscriber.seed_dict_listener().get_latest().event;
    assert!(matches!(seed_dict, DictionaryUpdate::Invalidate));

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

    // we change the round id to ensure that the state machine is
    // initialized with the coordinator state in the store
    // if we don't update the round_id we can't check if the state in the store was used or if the state was reset
    // because in both cases the round id will be 0
    let mut store = init_store().await;
    let mut state = CoordinatorState::new(pet_settings, mask_settings, model_settings.clone());
    let new_round_id = 5;
    state.round_id = new_round_id;
    store.set_coordinator_state(&state).await.unwrap();

    let smi = StateMachineInitializer::new(
        pet_settings,
        mask_settings,
        model_settings,
        RestoreSettings { enable: true },
        store,
    );

    let (state_machine, _request_sender, event_subscriber) = smi.init().await.unwrap();

    assert!(state_machine.is_idle());

    let phase = event_subscriber.phase_listener().get_latest().event;
    assert!(matches!(phase, PhaseName::Idle));

    let sum_dict = event_subscriber.sum_dict_listener().get_latest().event;
    assert!(matches!(sum_dict, DictionaryUpdate::Invalidate));

    let seed_dict = event_subscriber.seed_dict_listener().get_latest().event;
    assert!(matches!(seed_dict, DictionaryUpdate::Invalidate));

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

    let mut store = init_store().await;
    let mut state = CoordinatorState::new(pet_settings, mask_settings, model_settings.clone());
    let new_round_id = 7;
    state.round_id = new_round_id;
    store.set_coordinator_state(&state).await.unwrap();

    // upload a global model and set the id
    let uploaded_global_model = create_global_model(state.round_params.model_length);
    let global_model_id = store
        .set_global_model(
            state.round_id,
            &state.round_params.seed,
            &uploaded_global_model,
        )
        .await
        .unwrap();
    store
        .set_latest_global_model_id(&global_model_id)
        .await
        .unwrap();

    let smi = StateMachineInitializer::new(
        pet_settings,
        mask_settings,
        model_settings,
        RestoreSettings { enable: true },
        store,
    );

    let (state_machine, _request_sender, event_subscriber) = smi.init().await.unwrap();

    assert!(state_machine.is_idle());

    let phase = event_subscriber.phase_listener().get_latest().event;
    assert!(matches!(phase, PhaseName::Idle));

    let sum_dict = event_subscriber.sum_dict_listener().get_latest().event;
    assert!(matches!(sum_dict, DictionaryUpdate::Invalidate));

    let seed_dict = event_subscriber.seed_dict_listener().get_latest().event;
    assert!(matches!(seed_dict, DictionaryUpdate::Invalidate));

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

    let mut store = init_store().await;
    let mut state = CoordinatorState::new(pet_settings, mask_settings, model_settings.clone());
    let new_round_id = 9;
    state.round_id = new_round_id;
    store.set_coordinator_state(&state).await.unwrap();

    // upload a global model with a wrong model length and set the id
    let uploaded_global_model = create_global_model(state.round_params.model_length + 10);
    let global_model_id = store
        .set_global_model(
            state.round_id,
            &state.round_params.seed,
            &uploaded_global_model,
        )
        .await
        .unwrap();
    store
        .set_latest_global_model_id(&global_model_id)
        .await
        .unwrap();

    let smi = StateMachineInitializer::new(
        pet_settings,
        mask_settings,
        model_settings,
        RestoreSettings { enable: true },
        store,
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

    let mut store = init_store().await;
    let mut state = CoordinatorState::new(pet_settings, mask_settings, model_settings.clone());
    let new_round_id = 11;
    state.round_id = new_round_id;
    store.set_coordinator_state(&state).await.unwrap();

    // set a model id but don't store a model
    let global_model_id = "1_412957050209fcfa733b1fb4ad51f321";
    store
        .set_latest_global_model_id(&global_model_id)
        .await
        .unwrap();

    let smi = StateMachineInitializer::new(
        pet_settings,
        mask_settings,
        model_settings,
        RestoreSettings { enable: true },
        store,
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

    let mut store = init_store().await;
    let state = CoordinatorState::new(pet_settings, mask_settings, model_settings.clone());
    store.set_coordinator_state(&state).await.unwrap();

    let mut smi = StateMachineInitializer::new(
        pet_settings,
        mask_settings,
        model_settings,
        #[cfg(feature = "model-persistence")]
        RestoreSettings { enable: true },
        store.clone(),
    );

    smi.from_settings().await.unwrap();

    assert!(store.coordinator_state().await.unwrap().is_none());
    assert!(store.sum_dict().await.unwrap().is_none());
    assert!(store.seed_dict().await.unwrap().is_none());
    assert!(store.best_masks().await.unwrap().is_none());
    assert!(store.latest_global_model_id().await.unwrap().is_none());
    assert_eq!(store.number_of_unique_masks().await.unwrap(), 0);
}
