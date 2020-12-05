use thiserror::Error;
#[cfg(feature = "model-persistence")]
use tracing::{debug, info};

#[cfg(feature = "model-persistence")]
use crate::settings::RestoreSettings;
use crate::{
    settings::{MaskSettings, ModelSettings, PetSettings},
    state_machine::{
        coordinator::CoordinatorState,
        events::{EventPublisher, EventSubscriber, ModelUpdate},
        phases::{Idle, PhaseName, PhaseState, Shared},
        requests::{RequestReceiver, RequestSender},
        StateMachine,
    },
    storage::{CoordinatorStorage, ModelStorage, StorageError, Store},
};

#[cfg(feature = "model-persistence")]
use xaynet_core::mask::Model;

type StateMachineInitializationResult<T> = Result<T, StateMachineInitializationError>;

/// Error that can occur during the initialization of the [`StateMachine`].
#[derive(Debug, Error)]
pub enum StateMachineInitializationError {
    #[error("initializing crypto library failed")]
    CryptoInit,
    #[error("fetching coordinator state failed: {0}")]
    FetchCoordinatorState(StorageError),
    #[error("deleting coordinator data failed: {0}")]
    DeleteCoordinatorData(StorageError),
    #[error("fetching latest global model id failed: {0}")]
    FetchLatestGlobalModelId(StorageError),
    #[error("fetching global model failed: {0}")]
    FetchGlobalModel(StorageError),
    #[error("{0}")]
    GlobalModelUnavailable(String),
    #[error("{0}")]
    GlobalModelInvalid(String),
}

/// The state machine initializer that initializes a new state machine.
pub struct StateMachineInitializer<C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    pet_settings: PetSettings,
    mask_settings: MaskSettings,
    model_settings: ModelSettings,
    #[cfg(feature = "model-persistence")]
    restore_settings: RestoreSettings,

    store: Store<C, M>,
}

impl<C, M> StateMachineInitializer<C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Creates a new [`StateMachineInitializer`].
    pub fn new(
        pet_settings: PetSettings,
        mask_settings: MaskSettings,
        model_settings: ModelSettings,
        #[cfg(feature = "model-persistence")] restore_settings: RestoreSettings,
        store: Store<C, M>,
    ) -> Self {
        Self {
            pet_settings,
            mask_settings,
            model_settings,
            #[cfg(feature = "model-persistence")]
            restore_settings,
            store,
        }
    }

    #[cfg(not(feature = "model-persistence"))]
    /// Initializes a new [`StateMachine`] with the given settings.
    pub async fn init(
        mut self,
    ) -> StateMachineInitializationResult<(StateMachine<C, M>, RequestSender, EventSubscriber)>
    {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(StateMachineInitializationError::CryptoInit))?;

        let (coordinator_state, global_model) = { self.from_settings().await? };
        Ok(self.init_state_machine(coordinator_state, global_model))
    }

    // Creates a new [`CoordinatorState`] from the given settings and deletes
    // all coordinator data. Should only be called for the first start
    // or if we need to perform reset.
    pub(in crate::state_machine) async fn from_settings(
        &mut self,
    ) -> StateMachineInitializationResult<(CoordinatorState, ModelUpdate)> {
        self.store
            .delete_coordinator_data()
            .await
            .map_err(StateMachineInitializationError::DeleteCoordinatorData)?;
        Ok((
            CoordinatorState::new(
                self.pet_settings,
                self.mask_settings,
                self.model_settings.clone(),
            ),
            ModelUpdate::Invalidate,
        ))
    }

    // Initializes a new [`StateMachine`] with its components.
    fn init_state_machine(
        self,
        coordinator_state: CoordinatorState,
        global_model: ModelUpdate,
    ) -> (StateMachine<C, M>, RequestSender, EventSubscriber) {
        let (event_publisher, event_subscriber) = EventPublisher::init(
            coordinator_state.round_id,
            coordinator_state.keys.clone(),
            coordinator_state.round_params.clone(),
            PhaseName::Idle,
            global_model,
        );

        let (request_rx, request_tx) = RequestReceiver::new();

        let shared = Shared::new(coordinator_state, event_publisher, request_rx, self.store);

        let state_machine = StateMachine::from(PhaseState::<Idle, _, _>::new(shared));
        (state_machine, request_tx, event_subscriber)
    }
}

#[cfg(feature = "model-persistence")]
impl<C, M> StateMachineInitializer<C, M>
where
    C: CoordinatorStorage,
    M: ModelStorage,
{
    /// Initializes a new [`StateMachine`] by trying to restore the previous coordinator state
    /// along with the latest global model. After a successful initialization, the state machine
    /// always starts from a new round. This means that the round id is increased by one.
    /// If the state machine is reset during the initialization, the state machine starts
    /// with the round id `1`.
    ///
    /// # Behavior
    /// ![](https://mermaid.ink/svg/eyJjb2RlIjoic2VxdWVuY2VEaWFncmFtXG4gICAgYWx0IHJlc3RvcmUuZW5hYmxlID0gZmFsc2VcbiAgICAgICAgQ29vcmRpbmF0b3ItPj4rUmVkaXM6IGZsdXNoIGRiXG4gICAgICAgIE5vdGUgb3ZlciBDb29yZGluYXRvcixSZWRpczogc3RhcnQgZnJvbSBzZXR0aW5nc1xuICAgIGVsc2VcbiAgICAgICAgQ29vcmRpbmF0b3ItPj4rUmVkaXM6IGdldCBzdGF0ZVxuICAgICAgICBSZWRpcy0tPj4tQ29vcmRpbmF0b3I6IHN0YXRlXG4gICAgICAgIGFsdCBzdGF0ZSBub24tZXhpc3RlbnRcbiAgICAgICAgICAgIENvb3JkaW5hdG9yLT4-K1JlZGlzOiBmbHVzaCBkYlxuICAgICAgICAgICAgTm90ZSBvdmVyIENvb3JkaW5hdG9yLFJlZGlzOiBzdGFydCBmcm9tIHNldHRpbmdzXG4gICAgICAgIGVsc2Ugc3RhdGUgZXhpc3RcbiAgICAgICAgICAgIENvb3JkaW5hdG9yLT4-K1JlZGlzOiBnZXQgbGF0ZXN0IGdsb2JhbCBtb2RlbCBpZFxuICAgICAgICAgICAgUmVkaXMtLT4-LUNvb3JkaW5hdG9yOiBnbG9iYWwgbW9kZWwgaWRcbiAgICAgICAgICAgIGFsdCBnbG9iYWwgbW9kZWwgaWQgbm9uLWV4aXN0ZW50XG4gICAgICAgICAgICAgICAgTm90ZSBvdmVyIENvb3JkaW5hdG9yLFMzOiByZXN0b3JlIGNvb3JkaW5hdG9yIHdpdGggbGF0ZXN0IHN0YXRlIGJ1dCB3aXRob3V0IGEgZ2xvYmFsIG1vZGVsXG4gICAgICAgICAgICBlbHNlIGdsb2JhbCBtb2RlbCBpZCBleGlzdFxuICAgICAgICAgICAgICBDb29yZGluYXRvci0-PitTMzogZ2V0IGdsb2JhbCBtb2RlbFxuICAgICAgICAgICAgICBTMy0tPj4tQ29vcmRpbmF0b3I6IGdsb2JhbCBtb2RlbFxuICAgICAgICAgICAgICBhbHQgZ2xvYmFsIG1vZGVsIG5vbi1leGlzdGVudFxuICAgICAgICAgICAgICAgIE5vdGUgb3ZlciBDb29yZGluYXRvcixTMzogZXhpdCB3aXRoIGVycm9yXG4gICAgICAgICAgICAgIGVsc2UgZ2xvYmFsIG1vZGVsIGV4aXN0XG4gICAgICAgICAgICAgICAgTm90ZSBvdmVyIENvb3JkaW5hdG9yLFMzOiByZXN0b3JlIGNvb3JkaW5hdG9yIHdpdGggbGF0ZXN0IHN0YXRlIGFuZCBsYXRlc3QgZ2xvYmFsIG1vZGVsXG4gICAgICAgICAgICAgIGVuZFxuICAgICAgICAgICAgZW5kXG4gICAgICAgICAgZW5kXG4gICAgICAgIGVuZCIsIm1lcm1haWQiOnsidGhlbWUiOiJkZWZhdWx0IiwidGhlbWVWYXJpYWJsZXMiOnsiYmFja2dyb3VuZCI6IndoaXRlIiwicHJpbWFyeUNvbG9yIjoiI0VDRUNGRiIsInNlY29uZGFyeUNvbG9yIjoiI2ZmZmZkZSIsInRlcnRpYXJ5Q29sb3IiOiJoc2woODAsIDEwMCUsIDk2LjI3NDUwOTgwMzklKSIsInByaW1hcnlCb3JkZXJDb2xvciI6ImhzbCgyNDAsIDYwJSwgODYuMjc0NTA5ODAzOSUpIiwic2Vjb25kYXJ5Qm9yZGVyQ29sb3IiOiJoc2woNjAsIDYwJSwgODMuNTI5NDExNzY0NyUpIiwidGVydGlhcnlCb3JkZXJDb2xvciI6ImhzbCg4MCwgNjAlLCA4Ni4yNzQ1MDk4MDM5JSkiLCJwcmltYXJ5VGV4dENvbG9yIjoiIzEzMTMwMCIsInNlY29uZGFyeVRleHRDb2xvciI6IiMwMDAwMjEiLCJ0ZXJ0aWFyeVRleHRDb2xvciI6InJnYig5LjUwMDAwMDAwMDEsIDkuNTAwMDAwMDAwMSwgOS41MDAwMDAwMDAxKSIsImxpbmVDb2xvciI6IiMzMzMzMzMiLCJ0ZXh0Q29sb3IiOiIjMzMzIiwibWFpbkJrZyI6IiNFQ0VDRkYiLCJzZWNvbmRCa2ciOiIjZmZmZmRlIiwiYm9yZGVyMSI6IiM5MzcwREIiLCJib3JkZXIyIjoiI2FhYWEzMyIsImFycm93aGVhZENvbG9yIjoiIzMzMzMzMyIsImZvbnRGYW1pbHkiOiJcInRyZWJ1Y2hldCBtc1wiLCB2ZXJkYW5hLCBhcmlhbCIsImZvbnRTaXplIjoiMTZweCIsImxhYmVsQmFja2dyb3VuZCI6IiNlOGU4ZTgiLCJub2RlQmtnIjoiI0VDRUNGRiIsIm5vZGVCb3JkZXIiOiIjOTM3MERCIiwiY2x1c3RlckJrZyI6IiNmZmZmZGUiLCJjbHVzdGVyQm9yZGVyIjoiI2FhYWEzMyIsImRlZmF1bHRMaW5rQ29sb3IiOiIjMzMzMzMzIiwidGl0bGVDb2xvciI6IiMzMzMiLCJlZGdlTGFiZWxCYWNrZ3JvdW5kIjoiI2U4ZThlOCIsImFjdG9yQm9yZGVyIjoiaHNsKDI1OS42MjYxNjgyMjQzLCA1OS43NzY1MzYzMTI4JSwgODcuOTAxOTYwNzg0MyUpIiwiYWN0b3JCa2ciOiIjRUNFQ0ZGIiwiYWN0b3JUZXh0Q29sb3IiOiJibGFjayIsImFjdG9yTGluZUNvbG9yIjoiZ3JleSIsInNpZ25hbENvbG9yIjoiIzMzMyIsInNpZ25hbFRleHRDb2xvciI6IiMzMzMiLCJsYWJlbEJveEJrZ0NvbG9yIjoiI0VDRUNGRiIsImxhYmVsQm94Qm9yZGVyQ29sb3IiOiJoc2woMjU5LjYyNjE2ODIyNDMsIDU5Ljc3NjUzNjMxMjglLCA4Ny45MDE5NjA3ODQzJSkiLCJsYWJlbFRleHRDb2xvciI6ImJsYWNrIiwibG9vcFRleHRDb2xvciI6ImJsYWNrIiwibm90ZUJvcmRlckNvbG9yIjoiI2FhYWEzMyIsIm5vdGVCa2dDb2xvciI6IiNmZmY1YWQiLCJub3RlVGV4dENvbG9yIjoiYmxhY2siLCJhY3RpdmF0aW9uQm9yZGVyQ29sb3IiOiIjNjY2IiwiYWN0aXZhdGlvbkJrZ0NvbG9yIjoiI2Y0ZjRmNCIsInNlcXVlbmNlTnVtYmVyQ29sb3IiOiJ3aGl0ZSIsInNlY3Rpb25Ca2dDb2xvciI6InJnYmEoMTAyLCAxMDIsIDI1NSwgMC40OSkiLCJhbHRTZWN0aW9uQmtnQ29sb3IiOiJ3aGl0ZSIsInNlY3Rpb25Ca2dDb2xvcjIiOiIjZmZmNDAwIiwidGFza0JvcmRlckNvbG9yIjoiIzUzNGZiYyIsInRhc2tCa2dDb2xvciI6IiM4YTkwZGQiLCJ0YXNrVGV4dExpZ2h0Q29sb3IiOiJ3aGl0ZSIsInRhc2tUZXh0Q29sb3IiOiJ3aGl0ZSIsInRhc2tUZXh0RGFya0NvbG9yIjoiYmxhY2siLCJ0YXNrVGV4dE91dHNpZGVDb2xvciI6ImJsYWNrIiwidGFza1RleHRDbGlja2FibGVDb2xvciI6IiMwMDMxNjMiLCJhY3RpdmVUYXNrQm9yZGVyQ29sb3IiOiIjNTM0ZmJjIiwiYWN0aXZlVGFza0JrZ0NvbG9yIjoiI2JmYzdmZiIsImdyaWRDb2xvciI6ImxpZ2h0Z3JleSIsImRvbmVUYXNrQmtnQ29sb3IiOiJsaWdodGdyZXkiLCJkb25lVGFza0JvcmRlckNvbG9yIjoiZ3JleSIsImNyaXRCb3JkZXJDb2xvciI6IiNmZjg4ODgiLCJjcml0QmtnQ29sb3IiOiJyZWQiLCJ0b2RheUxpbmVDb2xvciI6InJlZCIsImxhYmVsQ29sb3IiOiJibGFjayIsImVycm9yQmtnQ29sb3IiOiIjNTUyMjIyIiwiZXJyb3JUZXh0Q29sb3IiOiIjNTUyMjIyIiwiY2xhc3NUZXh0IjoiIzEzMTMwMCIsImZpbGxUeXBlMCI6IiNFQ0VDRkYiLCJmaWxsVHlwZTEiOiIjZmZmZmRlIiwiZmlsbFR5cGUyIjoiaHNsKDMwNCwgMTAwJSwgOTYuMjc0NTA5ODAzOSUpIiwiZmlsbFR5cGUzIjoiaHNsKDEyNCwgMTAwJSwgOTMuNTI5NDExNzY0NyUpIiwiZmlsbFR5cGU0IjoiaHNsKDE3NiwgMTAwJSwgOTYuMjc0NTA5ODAzOSUpIiwiZmlsbFR5cGU1IjoiaHNsKC00LCAxMDAlLCA5My41Mjk0MTE3NjQ3JSkiLCJmaWxsVHlwZTYiOiJoc2woOCwgMTAwJSwgOTYuMjc0NTA5ODAzOSUpIiwiZmlsbFR5cGU3IjoiaHNsKDE4OCwgMTAwJSwgOTMuNTI5NDExNzY0NyUpIn19LCJ1cGRhdGVFZGl0b3IiOmZhbHNlfQ)
    ///
    /// - If the [`RestoreSettings.enable`] flag is set to `false`, the current coordinator
    ///   state will be reset and a new [`StateMachine`] is created with the given settings.
    /// - If no coordinator state exists, the current coordinator state will be reset and a new
    ///   [`StateMachine`] is created with the given settings.
    /// - If a coordinator state exists but no global model has been created so far, the
    ///   [`StateMachine`] will be restored with the coordinator state but without a global model.
    /// - If a coordinator state and a global model exists, the [`StateMachine`] will be restored
    ///   with the coordinator state and the global model.
    /// - If a global model has been created but does not exists, the initialization will fail with
    ///   [`StateMachineInitializationError::GlobalModelUnavailable`].
    /// - If a global model exists but its properties do not match the coordinator model settings,
    ///   the initialization will fail with [`StateMachineInitializationError::GlobalModelInvalid`].
    /// - Any network error will cause the initialization to fail.
    pub async fn init(
        mut self,
    ) -> StateMachineInitializationResult<(StateMachine<C, M>, RequestSender, EventSubscriber)>
    {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(StateMachineInitializationError::CryptoInit))?;

        let (coordinator_state, global_model) = if self.restore_settings.enable {
            self.from_previous_state().await?
        } else {
            info!("restoring coordinator state is disabled");
            info!("initialize state machine from settings");
            self.from_settings().await?
        };

        Ok(self.init_state_machine(coordinator_state, global_model))
    }

    // see [`StateMachineInitializer::init`]
    async fn from_previous_state(
        &mut self,
    ) -> StateMachineInitializationResult<(CoordinatorState, ModelUpdate)> {
        let (coordinator_state, global_model) = if let Some(coordinator_state) = self
            .store
            .coordinator_state()
            .await
            .map_err(StateMachineInitializationError::FetchCoordinatorState)?
        {
            self.try_restore_state(coordinator_state).await?
        } else {
            // no coordinator state available seems to be a fresh start
            self.from_settings().await?
        };

        Ok((coordinator_state, global_model))
    }

    // see [`StateMachineInitializer::init`]
    async fn try_restore_state(
        &mut self,
        coordinator_state: CoordinatorState,
    ) -> StateMachineInitializationResult<(CoordinatorState, ModelUpdate)> {
        let global_model_id = match self
            .store
            .latest_global_model_id()
            .await
            .map_err(StateMachineInitializationError::FetchLatestGlobalModelId)?
        {
            // the state machine was shut down before completing a round
            // we cannot use the round_id here because we increment the round_id after each restart
            // that means even if the round id is larger than one, it doesn't mean that a
            // round has ever been completed
            None => {
                debug!("apparently no round has been completed yet");
                debug!("restore coordinator without a global model");
                return Ok((coordinator_state, ModelUpdate::Invalidate));
            }
            Some(global_model_id) => global_model_id,
        };

        let global_model = self
            .load_global_model(&coordinator_state, &global_model_id)
            .await?;

        debug!(
            "restore coordinator with global model id: {}",
            global_model_id
        );
        Ok((
            coordinator_state,
            ModelUpdate::New(std::sync::Arc::new(global_model)),
        ))
    }

    // Loads a global model and checks its properties for suitability.
    async fn load_global_model(
        &mut self,
        coordinator_state: &CoordinatorState,
        global_model_id: &str,
    ) -> StateMachineInitializationResult<Model> {
        match self
            .store
            .global_model(&global_model_id)
            .await
            .map_err(StateMachineInitializationError::FetchGlobalModel)?
        {
            Some(global_model) => {
                if Self::model_properties_matches_settings(coordinator_state, &global_model) {
                    Ok(global_model)
                } else {
                    let error_msg = format!(
                        "the length of global model with the id {} does not match with the value of the model length setting {} != {}",
                        &global_model_id,
                        global_model.len(),
                        coordinator_state.round_params.model_length);

                    Err(StateMachineInitializationError::GlobalModelInvalid(
                        error_msg,
                    ))
                }
            }
            None => {
                // the model id exists but we cannot find it in the model store
                // here we better fail because if we restart a coordinator with an empty model
                // the clients will throw away their current global model and start from scratch
                Err(StateMachineInitializationError::GlobalModelUnavailable(
                    format!("cannot find global model {}", &global_model_id),
                ))
            }
        }
    }

    // Checks whether the properties of the loaded global model match the current
    // model settings of the coordinator.
    fn model_properties_matches_settings(
        coordinator_state: &CoordinatorState,
        global_model: &Model,
    ) -> bool {
        coordinator_state.round_params.model_length == global_model.len()
    }
}
