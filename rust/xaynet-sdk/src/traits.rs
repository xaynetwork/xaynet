use xaynet_core::{
    common::RoundParameters,
    mask::Model,
    SumDict,
    SumParticipantPublicKey,
    UpdateSeedDict,
};

/// A trait used by the [`StateMachine`] to emit notifications upon
/// certain events.
///
/// [`StateMachine`]: [crate::StateMachine]
pub trait Notify {
    /// Emit a notification when a new round of federated learning
    /// starts
    fn notify_new_round(&mut self) {}
    /// Emit a notification when the participant has been selected for
    /// the sum task
    fn notify_sum(&mut self) {}
    /// Emit a notification when the participant has been selected for
    /// the update task
    fn notify_update(&mut self) {}
    /// Emit a notification when the participant is not selected for
    /// any task and is waiting for another round to start
    fn notify_idle(&mut self) {}
}

/// A trait used by the [`StateMachine`] to load the model trained by
/// the participant, when it has been selected for the update task.
///
/// [`StateMachine`]: [crate::StateMachine]
#[async_trait]
pub trait ModelStore {
    type Error: ::std::error::Error;
    type Model: AsRef<Model> + Send;

    /// Attempt to load the model. If the model is not yet available,
    /// `Ok(None)` should be returned.
    async fn load_model(&mut self) -> Result<Option<Self::Model>, Self::Error>;
}

/// A trait used by the [`StateMachine`] to communicate with the
/// Xaynet coordinator.
///
/// [`StateMachine`]: [crate::StateMachine]
#[async_trait]
pub trait XaynetClient {
    type Error: ::std::error::Error;

    /// Retrieve the current round parameters
    async fn get_round_params(&mut self) -> Result<RoundParameters, Self::Error>;

    /// Retrieve the current sum dictionary, if available.
    async fn get_sums(&mut self) -> Result<Option<SumDict>, Self::Error>;

    /// Retrieve the current seed dictionary for the given sum
    /// participant, if available.
    async fn get_seeds(
        &mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<Option<UpdateSeedDict>, Self::Error>;

    /// Retrieve the current model/mask length, if available
    async fn get_mask_length(&mut self) -> Result<Option<u64>, Self::Error>;

    /// Retrieve the current global model, if available.
    async fn get_model(&mut self) -> Result<Option<Model>, Self::Error>;

    /// Send an encrypted and signed PET message to the coordinator.
    async fn send_message(&mut self, msg: Vec<u8>) -> Result<(), Self::Error>;
}
