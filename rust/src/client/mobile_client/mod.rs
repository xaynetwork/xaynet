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
    client_state: Option<ClientStateMachine>,
}

impl MobileClient {
    pub fn new(url: &str, participant_settings: ParticipantSettings) -> Self {
        let runtime = tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
            .unwrap();

        let local_model = None;
        let global_model = None;

        let client_state = ClientStateMachine::new(
            Proxy::new_remote(url),
            participant_settings,
            local_model,
            global_model,
        )
        .unwrap();

        Self {
            runtime,
            client_state: Some(client_state),
        }
    }

    pub fn set_local_model(&mut self, local_model: Model) {
        if let Some(ref mut client_state) = self.client_state {
            client_state.set_local_model(local_model);
        }
    }

    pub fn get_global_model(&self) -> Option<Model> {
        if let Some(client_state) = &self.client_state {
            client_state.get_global_model()
        } else {
            None
        }
    }

    pub fn next(&mut self) {
        if let Some(current_state) = self.client_state.take() {
            let new_state = self
                .runtime
                .block_on(async move { current_state.next().await });
            self.client_state = Some(new_state)
        }
    }

    pub fn create_participant_secret_key() -> SecretSigningKey {
        let SigningKeyPair { secret, .. } = SigningKeyPair::generate();
        secret
    }
}
