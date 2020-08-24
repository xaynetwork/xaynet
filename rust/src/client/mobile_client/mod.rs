pub mod client;
pub mod participant;

use crate::{
    client::{
        api::HttpApiClient,
        mobile_client::{
            client::{get_global_model, ClientStateMachine, LocalModel},
            participant::ParticipantSettings,
        },
    },
    crypto::{SecretSigningKey, SigningKeyPair},
    mask::model::Model,
};

pub struct MobileClient {
    api: HttpApiClient,
    local_model: LocalModelCache,
    client_state: ClientStateMachine,
}

impl MobileClient {
    pub fn init(url: &str, participant_settings: ParticipantSettings) -> Self {
        let client_state = ClientStateMachine::new(participant_settings).unwrap();
        Self::new(url, client_state)
    }

    pub fn deserialize(url: &str, bytes: &[u8]) -> Self {
        let client_state: ClientStateMachine = bincode::deserialize(bytes).unwrap();
        Self::new(url, client_state)
    }

    fn new(url: &str, client_state: ClientStateMachine) -> Self {
        let api = HttpApiClient::new(url);

        Self {
            api,
            client_state,
            local_model: LocalModelCache(None),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self.client_state).unwrap()
    }

    pub fn get_global_model(&mut self) -> Option<Model> {
        Self::runtime().block_on(async { get_global_model(&mut self.api).await })
    }

    pub fn perform_task(self) -> Self {
        let MobileClient {
            mut api,
            mut local_model,
            client_state,
        } = self;

        let client_state =
            Self::runtime().block_on(async { client_state.next(&mut api, &mut local_model).await });

        Self {
            api,
            local_model,
            client_state,
        }
    }

    pub fn set_local_model(&mut self, model: Model) {
        self.local_model.set_local_model(model);
    }

    pub fn create_participant_secret_key() -> SecretSigningKey {
        let SigningKeyPair { secret, .. } = SigningKeyPair::generate();
        secret
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap()
    }
}

struct LocalModelCache(Option<Model>);

impl LocalModelCache {
    fn set_local_model(&mut self, model: Model) {
        self.0 = Some(model);
    }
}

#[async_trait]
impl LocalModel for LocalModelCache {
    async fn get_local_model(&mut self) -> Option<Model> {
        self.0.clone()
    }
}
