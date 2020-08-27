pub mod client;
pub mod participant;

use crate::{
    api::{ApiClient, HttpApiClient, HttpApiClientError},
    mobile_client::{
        client::{ClientStateMachine, LocalModel},
        participant::ParticipantSettings,
    },
};
use thiserror::Error;
use xaynet_core::{
    crypto::{SecretSigningKey, SigningKeyPair},
    mask::Model,
    InitError,
};

#[derive(Debug, Error)]
/// Mobile client errors
pub enum MobileClientError {
    #[error("failed to deserialize mobile client: {0}")]
    /// Failed to deserialize mobile client.
    Deserialize(#[from] bincode::Error),
    #[error("failed to initialize crypto module: {0}")]
    /// Failed to initialize crypto module.
    Init(#[from] InitError),
    #[error("failed to initialize runtime: {0}")]
    /// Failed to initialize runtime.
    Runtime(#[from] std::io::Error),
    #[error("API request failed: {0}")]
    /// API request failed.
    Api(#[from] HttpApiClientError),
}

pub struct MobileClient {
    api: HttpApiClient,
    local_model: LocalModelCache,
    client_state: ClientStateMachine,
}

impl MobileClient {
    pub fn init(
        url: &str,
        participant_settings: ParticipantSettings,
    ) -> Result<Self, MobileClientError> {
        // It is critical that the initialization of sodiumoxide is successful.
        // We'd better not run the client than having a broken crypto.
        //
        // Refs:
        // https://doc.libsodium.org/usage
        // https://github.com/jedisct1/libsodium/issues/908
        let client_state = ClientStateMachine::new(participant_settings)?;
        Ok(Self::new(url, client_state))
    }

    pub fn restore(url: &str, bytes: &[u8]) -> Result<Self, MobileClientError> {
        let client_state: ClientStateMachine = bincode::deserialize(bytes)?;
        Ok(Self::new(url, client_state))
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
        // Safe to unwrap:
        //
        // - all sequences have known length
        //   - an iterator is an example for a sequence with an unknown length
        // - no untagged enum
        //
        // Refs:
        // - https://github.com/servo/bincode/issues/293
        // - https://github.com/servo/bincode/issues/255
        // - https://github.com/servo/bincode/issues/130#issuecomment-284641263
        bincode::serialize(&self.client_state).unwrap()
    }

    pub fn get_global_model(&mut self) -> Result<Option<Model>, MobileClientError> {
        Self::runtime()?
            .block_on(async { self.api.get_model().await })
            .map_err(|err| err.into())
    }

    pub fn try_to_proceed(self) -> Result<Self, (Self, MobileClientError)> {
        let mut runtime = match Self::runtime() {
            Ok(runtime) => runtime,
            // We don't want to loose the current client because of a runtime error.
            // Therefore we return the error as well as the current client.
            Err(err) => return Err((self, err.into())),
        };

        let MobileClient {
            mut api,
            mut local_model,
            client_state,
        } = self;

        let client_state =
            runtime.block_on(async { client_state.next(&mut api, &mut local_model).await });

        Ok(Self {
            api,
            local_model,
            client_state,
        })
    }

    /// Sets the local model.
    ///
    /// The local model is only sent if the client has been selected as an update client.
    /// If the client is an update client and no local model is available, the client remains
    /// in this state until a local model has been set or a new round has been started by the
    /// coordinator.
    pub fn set_local_model(&mut self, model: Model) {
        self.local_model.set_local_model(model);
    }

    pub fn create_participant_secret_key() -> SecretSigningKey {
        let SigningKeyPair { secret, .. } = SigningKeyPair::generate();
        secret
    }

    fn runtime() -> Result<tokio::runtime::Runtime, std::io::Error> {
        // Following the code of tokio, the creation of the I/O driver can result in an error.
        // It is not documented what exact condition can cause an error. Therefore we don't unwrap
        // here.
        tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()
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
