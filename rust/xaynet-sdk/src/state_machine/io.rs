use std::error::Error;

use async_trait::async_trait;

use xaynet_core::{
    common::RoundParameters,
    mask::Model,
    SumDict,
    SumParticipantPublicKey,
    UpdateSeedDict,
};

use crate::{ModelStore, Notify, XaynetClient};

/// Returned a dynamically dispatched [`IO`] object
pub(crate) fn boxed_io<X, M, N>(
    xaynet_client: X,
    model_store: M,
    notifier: N,
) -> Box<dyn IO<Model = Box<dyn AsRef<Model> + Send>>>
where
    X: XaynetClient + Send + 'static,
    M: ModelStore + Send + 'static,
    N: Notify + Send + 'static,
{
    Box::new(StateMachineIO::new(xaynet_client, model_store, notifier))
}

#[cfg(test)]
type DynModel = Box<(dyn std::convert::AsRef<xaynet_core::mask::Model> + Send)>;
/// A trait that gathers all the [`Notify`], [`XaynetClient`] and [`ModelStore`]
/// methods.
///
/// This trait is intended not to be exposed. It is a convenience for avoiding the
/// proliferation of generic parameters in the state machine: instead of three traits,
/// we now have only one.
///
/// Note that by having only one trait, we can also use dynamic dispatch and actually
/// get rid of all the generic parameters in the state machine.
///
/// ```ignore
/// Box<dyn IO> // allowed
/// Box<dyn ModelStore + Notify + XaynetClient // not allowed
/// ```
#[cfg_attr(test, mockall::automock(type Model=DynModel;))]
#[async_trait]
pub(crate) trait IO: Send + 'static {
    type Model;

    /// Attempt to load the model from the store.
    async fn load_model(&mut self) -> Result<Option<Self::Model>, Box<dyn Error>>;

    /// Fetch the round parameters from the coordinator
    async fn get_round_params(&mut self) -> Result<RoundParameters, Box<dyn Error>>;
    /// Fetch the sum dictionary from the coordinator
    async fn get_sums(&mut self) -> Result<Option<SumDict>, Box<dyn Error>>;
    /// Fetch the seed dictionary for the given sum participant from the coordinator
    async fn get_seeds(
        &mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<Option<UpdateSeedDict>, Box<dyn Error>>;
    /// Fetch the latest global model from the coordinator
    async fn get_model(&mut self) -> Result<Option<Model>, Box<dyn Error>>;
    /// Send the given signed and encrypted PET message to the coordinator
    async fn send_message(&mut self, msg: Vec<u8>) -> Result<(), Box<dyn Error>>;

    /// Notify the participant that a new round started
    fn notify_new_round(&mut self);
    /// Notify the participant that they have been selected for the sum task for the current
    /// round
    fn notify_sum(&mut self);
    /// Notify the participant that it is selected for the update task for the current
    /// round
    fn notify_update(&mut self);
    /// Notify the participant that is done with its current task and it waiting for
    /// being selected for a task
    fn notify_idle(&mut self);
    /// Notify the participant that is is expected to provide a model to the state
    /// machine by loading it into the store
    fn notify_load_model(&mut self);
}

/// Internal struct that implements the [`IO`] trait. It is not used as is in the state
/// machine. Instead, we box it and use it as a `dyn IO` object.
struct StateMachineIO<X, M, N> {
    xaynet_client: X,
    model_store: M,
    notifier: N,
}

impl<X, M, N> StateMachineIO<X, M, N> {
    /// Create a new `StateMachineIO`
    pub fn new(xaynet_client: X, model_store: M, notifier: N) -> Self {
        Self {
            xaynet_client,
            model_store,
            notifier,
        }
    }
}

#[async_trait]
impl<X, M, N> IO for StateMachineIO<X, M, N>
where
    X: XaynetClient + Send + 'static,
    M: ModelStore + Send + 'static,
    N: Notify + Send + 'static,
{
    type Model = Box<dyn AsRef<Model> + Send>;

    async fn load_model(&mut self) -> Result<Option<Self::Model>, Box<dyn Error>> {
        self.model_store
            .load_model()
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
            .map(|opt| opt.map(|model| Box::new(model) as Box<dyn AsRef<Model> + Send>))
    }

    async fn get_round_params(&mut self) -> Result<RoundParameters, Box<dyn Error>> {
        self.xaynet_client
            .get_round_params()
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    async fn get_sums(&mut self) -> Result<Option<SumDict>, Box<dyn Error>> {
        self.xaynet_client
            .get_sums()
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    async fn get_seeds(
        &mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<Option<UpdateSeedDict>, Box<dyn Error>> {
        self.xaynet_client
            .get_seeds(pk)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    async fn get_model(&mut self) -> Result<Option<Model>, Box<dyn Error>> {
        self.xaynet_client
            .get_model()
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    async fn send_message(&mut self, msg: Vec<u8>) -> Result<(), Box<dyn Error>> {
        self.xaynet_client
            .send_message(msg)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    fn notify_new_round(&mut self) {
        self.notifier.new_round()
    }

    fn notify_sum(&mut self) {
        self.notifier.sum()
    }

    fn notify_update(&mut self) {
        self.notifier.update()
    }

    fn notify_idle(&mut self) {
        self.notifier.idle()
    }

    fn notify_load_model(&mut self) {
        self.notifier.load_model()
    }
}

#[async_trait]
impl IO for Box<dyn IO<Model = Box<dyn AsRef<Model> + Send>>> {
    type Model = Box<dyn AsRef<Model> + Send>;

    async fn load_model(&mut self) -> Result<Option<Self::Model>, Box<dyn Error>> {
        self.as_mut().load_model().await
    }

    async fn get_round_params(&mut self) -> Result<RoundParameters, Box<dyn Error>> {
        self.as_mut().get_round_params().await
    }

    async fn get_sums(&mut self) -> Result<Option<SumDict>, Box<dyn Error>> {
        self.as_mut().get_sums().await
    }

    async fn get_seeds(
        &mut self,
        pk: SumParticipantPublicKey,
    ) -> Result<Option<UpdateSeedDict>, Box<dyn Error>> {
        self.as_mut().get_seeds(pk).await
    }

    async fn get_model(&mut self) -> Result<Option<Model>, Box<dyn Error>> {
        self.as_mut().get_model().await
    }

    async fn send_message(&mut self, msg: Vec<u8>) -> Result<(), Box<dyn Error>> {
        self.as_mut().send_message(msg).await
    }

    fn notify_new_round(&mut self) {
        self.as_mut().notify_new_round()
    }

    fn notify_sum(&mut self) {
        self.as_mut().notify_sum()
    }

    fn notify_update(&mut self) {
        self.as_mut().notify_update()
    }

    fn notify_idle(&mut self) {
        self.as_mut().notify_idle()
    }

    fn notify_load_model(&mut self) {
        self.as_mut().notify_load_model()
    }
}
