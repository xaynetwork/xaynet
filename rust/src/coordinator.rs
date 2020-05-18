use crate::{
    crypto::{generate_encrypt_key_pair, ByteObject, SigningKeySeed},
    mask::{Integers, Mask, MaskIntegers, MaskedModel},
    message::{sum::SumMessage, sum2::Sum2Message, update::UpdateMessage, Tag},
    model::Model,
    storage::store::{Connection, RedisStore},
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    InitError,
    LocalSeedDict,
    ParticipantTaskSignature,
    PetError,
    SeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
};
use async_trait::async_trait;
use derive_more::{AsMut, AsRef};
use futures::stream::{FuturesUnordered, StreamExt};
use redis::RedisError;
use sodiumoxide::{
    crypto::{box_, hash::sha256},
    randombytes::randombytes,
};
use std::{
    clone::Clone,
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    default::Default,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;

/// Error that occurs when the current round fails
#[derive(Debug, Eq, PartialEq)]
pub enum RoundFailed {
    /// Round failed because ambiguous masks were computed by the sum participants.
    AmbiguousMasks,
    /// Round failed because no mask was submitted by any sum participant.
    NoMask,
    /// Round failed because no model could be unmasked.
    NoModel,
}

/// A dictionary created during the sum2 phase of the protocol. It counts the model masks.
pub type MaskDict = HashMap<Mask, usize>;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
/// Round phases of a coordinator.
pub enum Phase {
    Idle,
    Sum,
    Update,
    Sum2,
}

/// Events the protocol emits.
#[derive(Debug, PartialEq)]
pub enum ProtocolEvent {
    /// The round starts with the given parameters. The coordinator is
    /// now in the sum phase.
    StartSum(RoundParameters),

    /// The sum phase finished and produced the given sum
    /// dictionary. The coordinator is now in the update phase.
    StartUpdate(SumDict),

    /// The update phase finished and produced the given seed
    /// dictionary. The coordinator is now in the sum2 phase.
    StartSum2(SeedDict),

    /// The sum2 phase finished and produced a global model. The
    /// coordinator is now back to the idle phase.
    EndRound(Option<()>),
}

#[derive(AsRef, AsMut, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// A seed for a round.
pub struct RoundSeed(box_::Seed);

impl ByteObject for RoundSeed {
    /// Create a round seed from a slice of bytes. Fails if the length of the input is invalid.
    fn from_slice(bytes: &[u8]) -> Option<Self> {
        box_::Seed::from_slice(bytes).map(Self)
    }

    /// Create a round seed initialized to zero.
    fn zeroed() -> Self {
        Self(box_::Seed([0_u8; Self::BYTES]))
    }

    /// Get the round seed as a slice.
    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl RoundSeed {
    /// Get the number of bytes of a round seed.
    pub const BYTES: usize = box_::SEEDBYTES;

    /// Generate a random round seed.
    pub fn generate() -> Self {
        // safe unwrap: length of slice is guaranteed by constants
        Self::from_slice_unchecked(randombytes(Self::BYTES).as_slice())
    }
}

/// A coordinator in the PET protocol layer.
pub struct Coordinator {
    // round parameters
    state: CoordinatorState,

    // redis store
    store: RedisStore,

    // Phase caches
    validation_cache: Option<Arc<ValidationCache>>,
    sum_phase_cache: Option<Arc<SumPhaseCache>>,

    // Message receiver
    msg_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
}

impl Coordinator {
    /// Create a coordinator. Fails if there is insufficient system entropy to generate secrets.
    pub async fn new(
        store: RedisStore,
    ) -> Result<(tokio::sync::mpsc::UnboundedSender<Vec<u8>>, Self), InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;

        let seed = RoundSeed::generate();
        let coordinator_state = CoordinatorState {
            seed,
            ..Default::default()
        };

        let (msg_tx, msg_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        let c = Self {
            state: coordinator_state.clone(),
            store,
            validation_cache: Some(Arc::new(ValidationCache::from_coordinator_state(
                coordinator_state,
            ))),
            sum_phase_cache: None,
            msg_rx,
        };

        c.set_state().await;

        Ok((msg_tx, c))
    }

    fn validation_cache(&self) -> Option<Arc<ValidationCache>> {
        self.validation_cache.clone()
    }

    fn sum_phase_cache(&self) -> Option<Arc<SumPhaseCache>> {
        // Safe after sum phase
        self.sum_phase_cache.clone()
    }

    /// Validate and handle a sum message.
    async fn handle_sum_message(
        redis: Connection,
        validation_cache: Arc<ValidationCache>,
        bytes: Vec<u8>,
    ) -> Result<Tag, PetError> {
        let msg = SumMessage::open(&bytes[..], &validation_cache.pk, &validation_cache.sk)?;
        msg.certificate().validate()?;
        Coordinator::validate_sum_task(validation_cache, msg.sum_signature(), msg.pk())?;
        Coordinator::add_sum_participant(redis, msg.pk(), msg.ephm_pk()).await?;
        Ok(Tag::Sum)
    }

    /// Validate a sum signature and its implied task.
    fn validate_sum_task(
        validation_cache: Arc<ValidationCache>,
        sum_signature: &ParticipantTaskSignature,
        pk: &SumParticipantPublicKey,
    ) -> Result<(), PetError> {
        if pk.verify_detached(
            sum_signature,
            &[validation_cache.seed.as_slice(), b"sum"].concat(),
        ) && sum_signature.is_eligible(validation_cache.sum)
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Add a sum participant to the sum dictionary. Fails if it is a repetition.
    async fn add_sum_participant(
        redis: Connection,
        pk: &SumParticipantPublicKey,
        ephm_pk: &SumParticipantEphemeralPublicKey,
    ) -> Result<(), PetError> {
        match redis.add_sum_participant(*pk, *ephm_pk).await {
            // field is new
            Ok(1) => Ok(()),
            // field already exists or redis error
            Ok(_) | Err(_) => Err(PetError::InvalidMessage),
        }
    }

    /// Validate and handle an update message.
    async fn handle_update_message(
        redis: Connection,
        validation_cache: Arc<ValidationCache>,
        sum_cache: Arc<SumPhaseCache>,
        bytes: Vec<u8>,
    ) -> Result<Tag, PetError> {
        let msg = UpdateMessage::open(&bytes[..], &validation_cache.pk, &validation_cache.sk)?;
        msg.certificate().validate()?;
        Coordinator::validate_update_task(
            validation_cache,
            msg.sum_signature(),
            msg.update_signature(),
            msg.pk(),
        )?;
        Coordinator::add_local_seed_dict(redis, sum_cache, msg.pk(), msg.local_seed_dict()).await?;
        Ok(Tag::Update)
    }

    /// Validate an update signature and its implied task.
    fn validate_update_task(
        validation_cache: Arc<ValidationCache>,
        sum_signature: &ParticipantTaskSignature,
        update_signature: &ParticipantTaskSignature,
        pk: &UpdateParticipantPublicKey,
    ) -> Result<(), PetError> {
        if pk.verify_detached(
            sum_signature,
            &[validation_cache.seed.as_slice(), b"sum"].concat(),
        ) && pk.verify_detached(
            update_signature,
            &[validation_cache.seed.as_slice(), b"update"].concat(),
        ) && !sum_signature.is_eligible(validation_cache.sum)
            && update_signature.is_eligible(validation_cache.update)
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Add a local seed dictionary to the seed dictionary. Fails if it contains invalid keys or it
    /// is a repetition.
    async fn add_local_seed_dict(
        redis: Connection,
        sum_cache: Arc<SumPhaseCache>,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> Result<(), PetError> {
        if local_seed_dict.keys().len() == sum_cache.len()
            && local_seed_dict
                .keys()
                .all(|pk| sum_cache.sum_pks().contains(pk))
        {
            redis
                .update_seed_dict(*pk, local_seed_dict.clone())
                .await
                .map_err(|_| PetError::InvalidMessage)
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Validate and handle a sum2 message.
    async fn handle_sum2_message(
        redis: RedisStore,
        validation_cache: Arc<ValidationCache>,
        bytes: Vec<u8>,
    ) -> Result<Tag, PetError> {
        let msg = Sum2Message::open(&bytes[..], &validation_cache.pk, &validation_cache.sk)?;
        msg.certificate().validate()?;
        Coordinator::validate_sum_task(validation_cache, msg.sum_signature(), msg.pk())?;
        Coordinator::add_mask(redis, msg.pk(), msg.mask()).await?;
        Ok(Tag::Sum2)
    }

    /// Add a mask to the mask dictionary. Fails if the sum participant didn't register in the sum
    /// phase or it is a repetition.
    async fn add_mask(
        redis: RedisStore,
        pk: &SumParticipantPublicKey,
        mask: &Mask,
    ) -> Result<(), PetError> {
        if let Ok(_) | Err(_) = redis
            .clone()
            .connection()
            .await
            .remove_sum_dict_entry(*pk)
            .await
        {
            // field does not exist or redis err
            return Err(PetError::InvalidMessage);
        }

        redis
            .connection()
            .await
            .incr_mask_count(mask.clone())
            .await
            .map_err(|_| PetError::InvalidMessage)
    }

    /// Clear the round dictionaries.
    async fn clear_round_dicts(&self) {
        self.store
            .clone()
            .connection()
            .await
            .flushdb()
            .await
            .unwrap();
    }

    /// Generate fresh round credentials.
    fn gen_round_keypair(&mut self) {
        let (pk, sk) = generate_encrypt_key_pair();
        self.state.pk = pk;
        self.state.sk = sk;
    }

    /// Update the round threshold parameters (dummy).
    fn update_round_thresholds(&self) {}

    /// Update the seed round parameter.
    fn update_round_seed(&mut self) {
        // safe unwrap: `sk` and `seed` have same number of bytes
        let (_, sk) = SigningKeySeed::from_slice_unchecked(self.state.sk.as_slice())
            .derive_signing_key_pair();
        let signature = sk.sign_detached(
            &[
                self.state.seed.as_slice(),
                &self.state.sum.to_le_bytes(),
                &self.state.update.to_le_bytes(),
            ]
            .concat(),
        );
        // safe unwrap: length of slice is guaranteed by constants
        self.state.seed =
            RoundSeed::from_slice_unchecked(sha256::hash(signature.as_slice()).as_ref());
    }

    /// Prepare the coordinator for a new round and go back to the initial phase.
    async fn start_new_round(&mut self) {
        self.clear_round_dicts().await;
        self.update_round_thresholds();
        self.update_round_seed();
        self.state.phase = Phase::Idle;
        self.set_state().await;
        self.sum_phase_cache = None;
    }

    /// End the sum2 phase and proceed to the idle phase to end the round.
    async fn proceed_idle_phase(&mut self) {
        info!("going to idle phase");
        self.start_new_round().await;
    }

    /// End the idle phase and proceed to the sum phase to start the round.
    async fn proceed_sum_phase(&mut self) {
        info!("going to sum phase");
        self.gen_round_keypair();
        self.state.phase = Phase::Sum;
        self.set_state().await;
    }

    /// End the sum phase and proceed to the update phase.
    async fn proceed_update_phase(&mut self) {
        info!("going to update phase");
        //self.freeze_sum_dict();
        self.state.phase = Phase::Update;
        self.set_state().await;
    }

    /// End the update phase and proceed to the sum2 phase.
    async fn proceed_sum2_phase(&mut self) {
        info!("going to sum2 phase");
        self.state.phase = Phase::Sum2;
        self.set_state().await;
    }

    async fn set_state(&self) {
        self.store
            .clone()
            .connection()
            .await
            .set_coordinator_state(self.state.clone())
            .await
            .unwrap();
    }

    /// Check whether enough sum participants submitted their ephemeral keys to start the update
    /// phase.
    fn has_enough_sums(&self) -> bool {
        self.state.sum_msg >= self.state.min_sum
    }

    /// Check whether enough update participants submitted their models and seeds to start the sum2
    /// phase.
    fn has_enough_seeds(&self) -> bool {
        self.state.update_msg >= self.state.min_update
    }

    /// Check whether enough sum participants submitted their masks to start the idle phase.
    fn has_enough_masks(&self) -> bool {
        self.state.mask_msg >= self.state.min_sum
    }

    /// Transition to the next phase if the protocol conditions are satisfied.
    async fn try_phase_transition(&mut self) {
        match self.state.phase {
            Phase::Idle => {
                self.proceed_sum_phase().await;
            }
            Phase::Sum => {
                if self.has_enough_sums() {
                    self.proceed_update_phase().await;
                }
            }
            Phase::Update => {
                if self.has_enough_seeds() {
                    self.proceed_sum2_phase().await;
                }
            }
            Phase::Sum2 => {
                if self.has_enough_masks() {
                    self.proceed_idle_phase().await;
                }
            }
        }
    }

    // Cancel the current round and restart a new one.
    // async fn reset(&mut self) {
    //     self.start_new_round().await;
    // }

    // Freeze the sum dictionary.
    // fn freeze_sum_dict(&mut self) {
    //     self.sum_phase_cache = Some(Arc::new(data));
    // }

    //// Freeze the mask dictionary.
    // fn freeze_mask_dict(&self) -> Result<&Mask, RoundFailed> {
    //     if self.mask_dict().is_empty() {
    //         Err(RoundFailed::NoMask)
    //     } else {
    //         let (mask, _) = self.mask_dict().iter().fold(
    //             (None, 0_usize),
    //             |(unique_mask, unique_count), (mask, count)| match unique_count.cmp(count) {
    //                 Ordering::Less => (Some(mask), *count),
    //                 Ordering::Greater => (unique_mask, unique_count),
    //                 Ordering::Equal => (None, unique_count),
    //             },
    //         );
    //         mask.ok_or(RoundFailed::AmbiguousMasks)
    //     }
    // }

    // fn round_parameters(&self) -> RoundParameters {
    //     RoundParameters {
    //         pk: *self.pk(),
    //         sum: *self.sum(),
    //         update: *self.update(),
    //         seed: self.seed().clone(),
    //     }
    // }
}

async fn coordinator_runner(mut coordinator: Coordinator) {
    let mut futures = FuturesUnordered::<JoinHandle<Result<Tag, PetError>>>::new();
    let mut batch_counter: u32 = 0;
    loop {
        let msg = match coordinator.msg_rx.recv().await {
            Some(mgs) => mgs,
            None => return,
        };

        match coordinator.state.phase {
            Phase::Idle => continue,
            Phase::Sum => {
                let fut = Coordinator::handle_sum_message(
                    coordinator.store.clone().connection().await,
                    coordinator.validation_cache().unwrap(),
                    msg,
                );
                futures.push(tokio::spawn(fut));
            }
            Phase::Update => {
                let fut = Coordinator::handle_update_message(
                    coordinator.store.clone().connection().await,
                    coordinator.validation_cache().unwrap(),
                    coordinator.sum_phase_cache().unwrap(),
                    msg,
                );
                futures.push(tokio::spawn(fut));
            }
            Phase::Sum2 => {
                let fut = Coordinator::handle_sum2_message(
                    coordinator.store.clone(),
                    coordinator.validation_cache().unwrap(),
                    msg,
                );
                futures.push(tokio::spawn(fut));
            }
        };

        batch_counter = batch_counter + 1;

        if batch_counter == 100 {
            // wait for all the requests to finish
            loop {
                match futures.next().await {
                    Some(Ok(Ok(tag))) => match tag {
                        Tag::Sum => coordinator.state.sum_msg += 1,
                        Tag::Update => coordinator.state.update_msg += 1,
                        Tag::Sum2 => coordinator.state.mask_msg += 1,
                        _ => unreachable!(),
                    },
                    None => break,
                    _ => continue,
                }
            }
        }
    }
}


#[derive(Clone)]
struct SumPhaseCache(HashSet<SumParticipantPublicKey>);

impl SumPhaseCache {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn sum_pks(&self) -> &HashSet<SumParticipantPublicKey> {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorState {
    pk: CoordinatorPublicKey,
    sk: CoordinatorSecretKey,
    sum: f64,
    update: f64,
    seed: RoundSeed,
    min_sum: usize,
    min_update: usize,
    phase: Phase,
    sum_msg: usize,
    update_msg: usize,
    mask_msg: usize,
}

impl CoordinatorState {}

impl Default for CoordinatorState {
    fn default() -> Self {
        Self {
            pk: CoordinatorPublicKey::zeroed(),
            sk: CoordinatorSecretKey::zeroed(),
            sum: 0.01_f64,
            update: 0.1_f64,
            seed: RoundSeed::zeroed(),
            min_sum: 1_usize,
            min_update: 3_usize,
            phase: Phase::Idle,
            sum_msg: 0,
            update_msg: 0,
            mask_msg: 0,
        }
    }
}

#[derive(Clone)]
pub struct ValidationCache {
    pk: CoordinatorPublicKey,
    sk: CoordinatorSecretKey,
    sum: f64,
    update: f64,
    seed: RoundSeed,
}

impl ValidationCache {
    pub fn from_coordinator_state(coordinator_state: CoordinatorState) -> Self {
        Self {
            pk: coordinator_state.pk,
            sk: coordinator_state.sk,
            sum: coordinator_state.sum,
            update: coordinator_state.update,
            seed: coordinator_state.seed,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct RoundParameters {
    /// The coordinator public key for encryption.
    pub pk: CoordinatorPublicKey,

    /// Fraction of participants to be selected for the sum task.
    pub sum: f64,

    /// Fraction of participants to be selected for the update task.
    pub update: f64,

    /// The random round seed.
    pub seed: RoundSeed,
}
