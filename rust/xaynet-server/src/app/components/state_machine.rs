use crate::state_machine::initializer::StateMachineInitializationError;
use crate::state_machine::StateMachine;
use crate::state_machine::StateMachineInitializer;
use crate::storage::Storage;
use crate::{
    settings::{MaskSettings, ModelSettings, PetSettings},
    state_machine::{events::EventSubscriber, requests::RequestSender},
};

pub async fn init<S: Storage>(
    pet_settings: PetSettings,
    mask_settings: MaskSettings,
    model_settings: ModelSettings,
    #[cfg(feature = "model-persistence")] restore_settings: RestoreSettings,
    store: S,
) -> Result<(StateMachine<S>, RequestSender, EventSubscriber), ()> {
    tracing::debug!("initialize");
    loop {
        match StateMachineInitializer::new(
            pet_settings,
            mask_settings,
            model_settings.clone(),
            #[cfg(feature = "model-persistence")]
            settings.restore.clone(),
            store.clone(),
        )
        .init()
        .await
        {
            Ok(state_machine) => break Ok(state_machine),
            Err(err) => {
                tracing::warn!("{}", err);
                if is_storage_error(&err) {
                    tokio::time::delay_for(tokio::time::Duration::from_secs(5)).await
                } else {
                    Err(())?
                }
            }
        }
    }
}

fn is_storage_error(err: &StateMachineInitializationError) -> bool {
    match err {
        StateMachineInitializationError::FetchCoordinatorState(_)
        | StateMachineInitializationError::DeleteCoordinatorData(_)
        | StateMachineInitializationError::FetchLatestGlobalModelId(_)
        | StateMachineInitializationError::FetchGlobalModel(_) => true,
        _ => false,
    }
}
