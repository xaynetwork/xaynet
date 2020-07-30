use crate::{
    client::Proxy,
    crypto::{SecretSigningKey, SigningKeyPair},
    mask::model::Model,
};

mod client;
use self::client::ClientStateMachine;

pub mod participant;
use self::participant::ParticipantSettings;

pub struct MobileClient {
    runtime: tokio::runtime::Runtime,
    client_state: ClientStateMachine,
}

impl MobileClient {
    pub fn new(url: &str, participant_settings: ParticipantSettings) -> Self {
        let runtime = tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap();

        let client_state =
            ClientStateMachine::new(Proxy::new_remote(url), participant_settings).unwrap();

        Self {
            runtime,
            client_state,
        }
    }

    pub fn set_local_model(&mut self, local_model: Model) {
        self.client_state.set_local_model(local_model);
    }

    pub fn get_global_model(&self) -> Option<Model> {
        self.client_state.get_global_model()
    }

    pub fn perform_task(self) -> Self {
        let Self {
            mut runtime,
            client_state,
        } = self;

        let new_client_state = runtime.block_on(async { client_state.next().await });

        Self {
            runtime,
            client_state: new_client_state,
        }
    }

    pub fn create_participant_secret_key() -> SecretSigningKey {
        let SigningKeyPair { secret, .. } = SigningKeyPair::generate();
        secret
    }
}
