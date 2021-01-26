use crate::{
    settings::{MaskSettings, PetSettings},
    state_machine::{
        events::DictionaryUpdate,
        phases::{Idle, Phase, PhaseName, PhaseState, Shared},
        requests::{UserRequest, UserResponseSender},
        PhaseStateError,
        StateMachine,
    },
    storage::Storage,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, error_span, info, warn};
use xaynet_core::mask::{MaskConfig, MaskConfigPair};

pub struct Interrupt {
    req: Option<(UserRequest, UserResponseSender)>,
}

#[derive(Deserialize)]
pub struct ConfigUpdate {
    pet: PetSettings,
    mask: MaskSettings,
}

#[async_trait]
impl<S> Phase<S> for PhaseState<Interrupt, S>
where
    S: Storage,
{
    const NAME: PhaseName = PhaseName::Interrupt;

    async fn run(&mut self) -> Result<(), PhaseStateError> {
        self.handle_request().await;
        Ok(())
    }

    fn next(self) -> Option<StateMachine<S>> {
        Some(PhaseState::<Idle, _>::new(self.shared).into())
    }
}

impl<S> PhaseState<Interrupt, S>
where
    S: Storage,
{
    pub fn new(shared: Shared<S>, req: (UserRequest, UserResponseSender)) -> Self {
        Self {
            private: Interrupt { req: Some(req) },
            shared,
        }
    }

    async fn handle_request(&mut self) {
        match self.private.req.take().unwrap() {
            (UserRequest::Pause, resp) => self.pause(resp).await,
            // do nothing
            (UserRequest::Resume, resp) => {
                info!("resume");
                let _ = resp.send(());
            }
            (UserRequest::Change(config), resp) => self.update_config(config, resp),
        }
    }

    // invalid dicts
    // only provide round_param and global_model
    // this future never completes
    // it will be dropped in `self.run_phase` if a new user request was sent
    async fn pause(&mut self, resp: UserResponseSender) {
        info!("pause");
        info!("broadcasting invalidation of sum dictionary from previous round");
        self.shared
            .events
            .broadcast_sum_dict(DictionaryUpdate::Invalidate);

        info!("broadcasting invalidation of seed dictionary from previous round");
        self.shared
            .events
            .broadcast_seed_dict(DictionaryUpdate::Invalidate);

        let _ = resp.send(());

        futures::future::pending::<()>().await
    }

    fn update_config(&mut self, config: ConfigUpdate, resp: UserResponseSender) {
        info!("update");
        // what is not safe to update
        // model length -> will fail on client side if the length of local and global model are different
        // global model (partially) -> will fail on client side if the length or datatype is different -> same length & data type should be OK
        // mask config data type -> will fail on client side if the datatype of local and global model are different

        // what is safe to update
        // all pet settings
        // mask config model type
        // mask config bound type
        // mask config group_type type
        //
        let ConfigUpdate { pet, mask } = config;

        let mask_conf: MaskConfigPair = MaskConfig::from(mask).into();

        self.shared.state.sum = pet.sum.into();
        self.shared.state.update = pet.update.into();
        self.shared.state.sum2 = pet.sum2.into();
        self.shared.state.round_params.mask_config.vect.group_type = mask_conf.vect.group_type;
        self.shared.state.round_params.mask_config.vect.bound_type = mask_conf.vect.bound_type;
        self.shared.state.round_params.mask_config.vect.model_type = mask_conf.vect.model_type;
        let _ = resp.send(());
    }
}
