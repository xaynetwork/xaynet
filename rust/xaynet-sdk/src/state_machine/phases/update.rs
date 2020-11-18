use std::ops::Deref;

use async_trait::async_trait;
use derive_more::From;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use xaynet_core::{
    crypto::Signature,
    mask::{MaskObject, MaskSeed, Masker, Model},
    message::Update as UpdateMessage,
    LocalSeedDict,
    ParticipantTaskSignature,
    SumDict,
};

use crate::{
    state_machine::{IntoPhase, Phase, PhaseIo, Progress, State, Step, TransitionOutcome, IO},
    MessageEncoder,
};

#[derive(From)]
pub enum LocalModel {
    Dyn(Box<dyn AsRef<Model> + Send>),
    Owned(Model),
}

impl std::fmt::Debug for LocalModel {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LocalModel::Dyn(_) => fmt.debug_tuple("LocalModel::Dyn"),
            LocalModel::Owned(_) => fmt.debug_tuple("LocalModel::Owned"),
        }
        .field(&"...")
        .finish()
    }
}

impl AsRef<Model> for LocalModel {
    fn as_ref(&self) -> &Model {
        match self {
            LocalModel::Dyn(model) => model.deref().as_ref(),
            LocalModel::Owned(model) => model,
        }
    }
}

impl serde::ser::Serialize for LocalModel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            LocalModel::Dyn(model) => model.as_ref().as_ref().serialize(serializer),
            LocalModel::Owned(model) => model.serialize(serializer),
        }
    }
}

impl<'de> serde::de::Deserialize<'de> for LocalModel {
    fn deserialize<D>(deserializer: D) -> Result<LocalModel, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let model = <Model as serde::de::Deserialize>::deserialize(deserializer)?;
        Ok(LocalModel::Owned(model))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Update {
    pub sum_signature: ParticipantTaskSignature,
    pub update_signature: ParticipantTaskSignature,
    pub sum_dict: Option<SumDict>,
    pub seed_dict: Option<LocalSeedDict>,
    pub model: Option<LocalModel>,
    pub mask: Option<(MaskSeed, MaskObject)>,
    pub message: Option<MessageEncoder>,
}

impl Update {
    pub fn new(sum_signature: Signature, update_signature: Signature) -> Self {
        Update {
            sum_signature,
            update_signature,
            sum_dict: None,
            seed_dict: None,
            model: None,
            mask: None,
            message: None,
        }
    }

    fn has_fetched_sum_dict(&self) -> bool {
        self.sum_dict.is_some() || self.has_loaded_model()
    }

    fn has_loaded_model(&self) -> bool {
        self.model.is_some() || self.has_masked_model()
    }

    fn has_masked_model(&self) -> bool {
        self.mask.is_some() || self.has_built_seed_dict()
    }

    fn has_built_seed_dict(&self) -> bool {
        self.seed_dict.is_some() || self.has_composed_message()
    }

    fn has_composed_message(&self) -> bool {
        self.message.is_some()
    }
}

impl IntoPhase<Update> for State<Update> {
    fn into_phase(self, mut io: PhaseIo) -> Phase<Update> {
        io.notify_update();
        if !self.private.has_loaded_model() {
            io.notify_load_model();
        }
        Phase::<_>::new(self, io)
    }
}

#[async_trait]
impl Step for Phase<Update> {
    async fn step(mut self) -> TransitionOutcome {
        self = try_progress!(self.fetch_sum_dict().await);
        self = try_progress!(self.load_model().await);
        self = try_progress!(self.mask_model());
        self = try_progress!(self.build_seed_dict());
        self = try_progress!(self.compose_update_message());

        // FIXME: currently if sending fails, we lose the message,
        // thus wasting all the work we've done in this phase
        //
        // UNWRAP_SAFE: the message is set in
        // `self.compose_update_message()`
        let message = self.state.private.message.take().unwrap();
        match self.send_message(message).await {
            Ok(_) => {
                info!("sent update message");
            }
            Err(e) => {
                warn!("failed to send update message: {}", e);
                warn!("update phase failed");
            }
        }

        info!("going back to awaiting phase");
        TransitionOutcome::Complete(self.into_awaiting().into())
    }
}

impl Phase<Update> {
    async fn fetch_sum_dict(mut self) -> Progress<Update> {
        if self.state.private.has_fetched_sum_dict() {
            debug!("already fetched the sum dictionary, continuing");
            return Progress::Continue(self);
        }
        debug!("fetching sum dictionary");
        match self.io.get_sums().await {
            Ok(Some(dict)) => {
                self.state.private.sum_dict = Some(dict);
                Progress::Updated(self.into())
            }
            Ok(None) => {
                debug!("sum dictionary is not available yet");
                Progress::Stuck(self)
            }
            Err(e) => {
                warn!("failed to fetch sum dictionary: {:?}", e);
                Progress::Stuck(self)
            }
        }
    }

    async fn load_model(mut self) -> Progress<Update> {
        if self.state.private.has_loaded_model() {
            debug!("already loaded the model, continuing");
            return Progress::Continue(self);
        }

        debug!("loading local model");
        match self.io.load_model().await {
            Ok(Some(model)) => {
                self.state.private.model = Some(model.into());
                Progress::Updated(self.into())
            }
            Ok(None) => {
                debug!("model is not ready");
                Progress::Stuck(self)
            }
            Err(e) => {
                warn!("failed to load model: {:?}", e);
                Progress::Stuck(self)
            }
        }
    }

    /// Generate a mask seed and mask a local model.
    fn mask_model(mut self) -> Progress<Update> {
        if self.state.private.has_masked_model() {
            debug!("already computed the masked model, continuing");
            return Progress::Continue(self);
        }
        info!("computing masked model");
        let config = self.state.shared.mask_config;
        let masker = Masker::new(config);
        // UNWRAP_SAFE: the model is set, per the `has_masked_model()`
        // check above
        let model = self.state.private.model.take().unwrap();
        let scalar = self.state.shared.scalar;
        self.state.private.mask = Some(masker.mask(scalar, model.as_ref()));
        Progress::Updated(self.into())
    }

    // Create a local seed dictionary from a sum dictionary.
    fn build_seed_dict(mut self) -> Progress<Update> {
        if self.state.private.has_built_seed_dict() {
            debug!("already built the seed dictionary, continuing");
            return Progress::Continue(self);
        }
        // UNWRAP_SAFE: the mask is set `self.mask_model()` which is
        // called before this method.
        let mask_seed = &self.state.private.mask.as_ref().unwrap().0;
        info!("building local seed dictionary");
        let seeds = self
            .state
            .private
            .sum_dict
            .take()
            .unwrap()
            .into_iter()
            .map(|(pk, ephm_pk)| (pk, mask_seed.encrypt(&ephm_pk)))
            .collect();
        self.state.private.seed_dict = Some(seeds);
        Progress::Updated(self.into())
    }

    fn compose_update_message(mut self) -> Progress<Update> {
        if self.state.private.has_composed_message() {
            debug!("already composed the update message, continuing");
            return Progress::Continue(self);
        }
        debug!("composing update message");
        let update = UpdateMessage {
            sum_signature: self.state.private.sum_signature,
            update_signature: self.state.private.update_signature,
            // UNWRAP_SAFE: the mask is set in `self.mask_model()`
            // which is called before this method
            masked_model: self.state.private.mask.take().unwrap().1,
            // UNWRAP_SAFE: the mask is set in
            // `self.build_seed_dict()` which is called before this
            // method
            local_seed_dict: self.state.private.seed_dict.take().unwrap(),
        };
        self.state.private.message = Some(self.message_encoder(update.into()));
        Progress::Updated(self.into())
    }
}

#[cfg(test)]
mod tests {
    use mockall::Sequence;
    use xaynet_core::{
        crypto::ByteObject,
        mask::{FromPrimitives, Model},
        SumDict,
    };

    use super::*;
    use crate::{
        state_machine::{
            phases::Awaiting,
            testutils::{shared_state, EncryptKeyGenerator, SelectFor, SigningKeyGenerator},
            MockIO,
            SharedState,
        },
        unwrap_progress_continue,
        unwrap_step,
    };

    /// Instantiate a sum phase.
    fn make_phase() -> Phase<Update> {
        let shared = shared_state(SelectFor::Sum);
        let update = make_update(&shared);

        // Check IntoPhase<Update> implementation
        let mut mock = MockIO::new();
        mock.expect_notify_update().times(1).return_const(());
        mock.expect_notify_load_model().times(1).return_const(());
        let mut phase: Phase<Update> = State::new(shared, update).into_phase(Box::new(mock));

        // Drop phase.io to force the mock checks to run now
        let _ = std::mem::replace(&mut phase.io, Box::new(MockIO::new()));
        phase
    }

    fn make_update(shared: &SharedState) -> Update {
        let sk = &shared.keys.secret;
        let seed = shared.round_params.seed.as_slice();
        let sum_signature = sk.sign_detached(&[seed, b"sum"].concat());
        let update_signature = sk.sign_detached(&[seed, b"update"].concat());
        Update {
            sum_signature,
            update_signature,
            sum_dict: None,
            seed_dict: None,
            model: None,
            mask: None,
            message: None,
        }
    }

    fn make_model() -> Model {
        let weights: Vec<f32> = vec![1.1, 2.2, 3.3, 4.4];
        Model::from_primitives(weights.into_iter()).unwrap()
    }

    fn make_sum_dict() -> SumDict {
        let mut dict = SumDict::new();

        let mut signing_keys = SigningKeyGenerator::new();
        let mut encrypt_keys = EncryptKeyGenerator::new();

        dict.insert(signing_keys.next().public, encrypt_keys.next().public);
        dict.insert(signing_keys.next().public, encrypt_keys.next().public);

        dict
    }

    async fn step1_fetch_sum_dict(mut phase: Phase<Update>) -> Phase<Update> {
        let mut io = MockIO::new();
        let mut seq = Sequence::new();
        // The first time the state machine fetches the sum dict,
        // pretend it's not publiches yet
        io.expect_get_sums()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(None));
        // The second time, return a sum dictionary.
        io.expect_get_sums()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(Some(make_sum_dict())));
        phase.io = Box::new(io);

        // First time: no progress should be made, since we didn't
        // fetch any sum dict yet
        let phase = unwrap_step!(phase, pending, update);
        assert!(!phase.state.private.has_fetched_sum_dict());

        // Second time: now the state machine should have made progress
        let phase = unwrap_step!(phase, complete, update);
        assert!(phase.state.private.has_fetched_sum_dict());

        // Calling `fetch_sum_dict` again should return Progress::Continue
        let phase = unwrap_progress_continue!(phase, fetch_sum_dict, async);
        io_checks(phase)
    }

    async fn step2_load_model(mut phase: Phase<Update>) -> Phase<Update> {
        let mut io = MockIO::new();
        let mut seq = Sequence::new();
        // The first time the state machine fetches the sum dict,
        // pretend it's not publiches yet
        io.expect_load_model()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(None));
        // The second time, return a sum dictionary.
        io.expect_load_model()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(Some(Box::new(make_model()))));
        phase.io = Box::new(io);

        // First time: no progress should be made, since we didn't
        // load any model
        let phase = unwrap_step!(phase, pending, update);
        assert!(phase.state.private.has_fetched_sum_dict());
        assert!(!phase.state.private.has_loaded_model());

        // Second time: now the state machine should have made progress
        let phase = unwrap_step!(phase, complete, update);
        assert!(phase.state.private.has_loaded_model());

        // Calling `load_model` again should return Progress::Continue
        let phase = unwrap_progress_continue!(phase, load_model, async);
        io_checks(phase)
    }

    fn io_checks<T>(mut phase: Phase<T>) -> Phase<T> {
        // Drop phase.io to force the mock checks to run now
        let _ = std::mem::replace(&mut phase.io, Box::new(MockIO::new()));
        phase
    }

    async fn step3_mask_model(phase: Phase<Update>) -> Phase<Update> {
        let phase = unwrap_step!(phase, complete, update);
        assert!(phase.state.private.has_masked_model());
        let phase = unwrap_progress_continue!(phase, mask_model);
        io_checks(phase)
    }

    async fn step4_build_seed_dict(phase: Phase<Update>) -> Phase<Update> {
        let phase = unwrap_step!(phase, complete, update);
        assert!(phase.state.private.has_built_seed_dict());
        let phase = unwrap_progress_continue!(phase, build_seed_dict);
        io_checks(phase)
    }

    async fn step5_compose_update_message(phase: Phase<Update>) -> Phase<Update> {
        let phase = unwrap_step!(phase, complete, update);
        assert!(phase.state.private.has_composed_message());
        let phase = unwrap_progress_continue!(phase, compose_update_message);
        io_checks(phase)
    }

    async fn step6_send_message(mut phase: Phase<Update>) -> Phase<Awaiting> {
        let mut io = MockIO::new();
        let mut seq = Sequence::new();
        io.expect_send_message()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        io.expect_notify_idle()
            .times(1)
            .in_sequence(&mut seq)
            .return_const(());
        phase.io = Box::new(io);
        let phase = unwrap_step!(phase, complete, awaiting);
        io_checks(phase)
    }

    #[tokio::test]
    async fn test_update_phase() {
        let phase = make_phase();
        let phase = step1_fetch_sum_dict(phase).await;
        let phase = step2_load_model(phase).await;
        let phase = step3_mask_model(phase).await;
        let phase = step4_build_seed_dict(phase).await;
        let phase = step5_compose_update_message(phase).await;
        step6_send_message(phase).await;
    }
}
