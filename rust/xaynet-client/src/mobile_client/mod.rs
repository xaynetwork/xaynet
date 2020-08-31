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
    /// Initializes a fresh client. This method only needs to be called once.
    ///
    /// To serialize and restore a client use the [`MobileClient::serialize`] and
    /// [`MobileClient::restore`]
    ///
    /// # Errors
    ///
    /// Fails if the crypto module cannot be initialized.
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

    /// Restores a client from its serialized state.
    ///
    /// # Errors
    ///
    /// Fails if the serialized state is corrupted and the client cannot be restored
    /// or if the crypto module cannot be initialized.
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

    /// Serializes the current state of the client.
    ///
    /// # Note
    ///
    /// The serialized state is **not encrypted** and contains sensitive data such as the
    /// participant's private key. Therefore, the user of the [`MobileClient`] **must** ensure
    /// that the serialized state is stored in a safe place.
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

    /// Fetches and returns the latest global model from the coordinator.
    /// Returns `None` if no global model is available.
    ///
    /// # Errors
    ///
    /// Fails if the runtime cannot be initialized or if an API request has failed.
    pub fn get_global_model(&mut self) -> Result<Option<Model>, MobileClientError> {
        Self::runtime()?
            .block_on(async { self.api.get_model().await })
            .map_err(|err| err.into())
    }

    /// Tries to proceed with the current client task.
    /// This will consume the current state of the client and produces a new one.
    ///
    /// # Errors
    ///
    /// Fails if the runtime cannot be initialized.
    /// In this case the state of the client remains unchanged and is returned
    /// along with the error.
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

    /// Returns the current state of the client.
    pub fn get_current_state(&self) -> ClientStateName {
        match self.client_state {
            ClientStateMachine::Awaiting(_) => ClientStateName::Awaiting,
            ClientStateMachine::Sum(_) => ClientStateName::Sum,
            ClientStateMachine::Update(_) => ClientStateName::Update,
            ClientStateMachine::Sum2(_) => ClientStateName::Sum2,
        }
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

    /// Creates a new participant secret key.
    ///
    /// The secret key is part of the [`ParticipantSettings`] which are required for the first
    /// initialization of the client.
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

pub enum ClientStateName {
    Awaiting,
    Sum,
    Update,
    Sum2,
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
