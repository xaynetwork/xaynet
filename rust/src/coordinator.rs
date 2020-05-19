use crate::{
    crypto::{generate_encrypt_key_pair, ByteObject, SigningKeySeed},
    mask::{Integers, Mask, MaskIntegers, MaskedModel},
    message::{sum::SumMessage, sum2::Sum2Message, update::UpdateMessage},
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
    task::{Context, Poll},
};
use tokio::{
    sync::{
        broadcast,
        mpsc,
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Semaphore,
    },
    task::JoinHandle,
};

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
    // Local coordinator
    state: CoordinatorState,

    // Redis store
    store: RedisStore,

    // Caches
    validation_cache: Option<Arc<ValidationCache>>,
    sum_phase_cache: Option<Arc<SumPhaseCache>>,

    // Message receiver
    msg_rx: UnboundedReceiver<Vec<u8>>,

    // Semaphore to limit the messages that can run concurrently
    limit_msg_processing: Arc<Semaphore>,
}

impl Coordinator {
    /// Create a coordinator. Fails if there is insufficient system entropy to generate secrets.
    pub async fn new(store: RedisStore) -> Result<(UnboundedSender<Vec<u8>>, Self), InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;

        let seed = RoundSeed::generate();
        let coordinator_state = CoordinatorState {
            seed,
            ..Default::default()
        };

        let (msg_tx, msg_rx) = unbounded_channel::<Vec<u8>>();

        let c = Self {
            state: coordinator_state.clone(),
            store,
            validation_cache: Some(Arc::new(ValidationCache::from_coordinator_state(
                coordinator_state,
            ))),
            sum_phase_cache: None,
            msg_rx,
            limit_msg_processing: Arc::new(Semaphore::new(0)),
        };

        c.clear_redis_state().await;
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
        self.clear_redis_state().await; // remove only the dicts
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

        // We want to process as many messages as possible at the same time but we don't want to
        // process more than is necessary (min_sum).
        // The important point here is, that we always have to "await" all spawned futures.
        // If we do not await them, it can happen that the coordinator moves on to the next phase,
        // and in the background, futures of the previous phase will still change the data of the
        // previous phase. This can lead to the fact, that the local data state does not match
        // the Redis data state.
        self.limit_msg_processing = Arc::new(Semaphore::new(self.state.min_sum));
        let (success_tx, counter_fut) = MsgCounter::new(self.state.min_sum);

        tokio::select! {
            // A future that never resolves
            _ = async {
                loop {
                    self.limit_msg_processing.acquire().await.forget();

                    let msg = match self.msg_rx.recv().await {
                        Some(mgs) => mgs,
                        None => return,
                    };

                    let fut = MsgHandler::handle_sum_message(
                        self.store.clone().connection().await,
                        self.validation_cache().unwrap(),
                        msg,
                    );

                    let handler = MsgHandler {
                        success_tx: success_tx.clone(),
                        limit_msg_processing: self.limit_msg_processing.clone(),
                    };

                    tokio::spawn(async move { handler.run(fut).await });
                }
            } => {panic!("message sender dropped")}
            _ = counter_fut  => {
                info!("sum phase complete");
            }
        }
    }

    /// End the sum phase and proceed to the update phase.
    async fn proceed_update_phase(&mut self) {
        info!("going to update phase");
        self.freeze_sum_dict().await;
        self.state.phase = Phase::Update;
        self.set_state().await;
        self.limit_msg_processing = Arc::new(Semaphore::new(self.state.min_update));
        let (success_tx, counter_fut) = MsgCounter::new(self.state.min_update);

        tokio::select! {
            _ = async {
                loop {
                    self.limit_msg_processing.acquire().await.forget();

                    let msg = match self.msg_rx.recv().await {
                        Some(mgs) => mgs,
                        None => return,
                    };

                    let fut = MsgHandler::handle_update_message(
                        self.store.clone().connection().await,
                        self.validation_cache().unwrap(),
                        self.sum_phase_cache().unwrap(),
                        msg,
                    );

                    let handler = MsgHandler {
                        success_tx: success_tx.clone(),
                        limit_msg_processing: self.limit_msg_processing.clone(),
                    };

                    tokio::spawn(async move { handler.run(fut).await });
                }
            } => {
                panic!("message sender dropped")
            }
            _ = counter_fut  => {
                info!("update phase complete");
            }
        }
    }

    /// End the update phase and proceed to the sum2 phase.
    async fn proceed_sum2_phase(&mut self) {
        info!("going to sum2 phase");
        self.state.phase = Phase::Sum2;
        self.set_state().await;
        self.limit_msg_processing = Arc::new(Semaphore::new(self.state.min_sum));
        let (success_tx, counter_fut) = MsgCounter::new(self.state.min_sum);

        tokio::select! {
            _ = async {
                loop {
                    self.limit_msg_processing.acquire().await.forget();

                    let msg = match self.msg_rx.recv().await {
                        Some(mgs) => mgs,
                        None => return,
                    };

                    let fut = MsgHandler::handle_sum2_message(
                        self.store.clone(),
                        self.validation_cache().unwrap(),
                        msg,
                    );

                    let handler = MsgHandler {
                        success_tx: success_tx.clone(),
                        limit_msg_processing: self.limit_msg_processing.clone(),
                    };

                    tokio::spawn(async move { handler.run(fut).await });
                }
            } => {
                panic!("message sender dropped")
            }
            _ = counter_fut  => {
                info!("sum2 phase complete");
            }
        }
    }

    /// Write the local state in redis
    async fn set_state(&self) {
        self.store
            .clone()
            .connection()
            .await
            .set_coordinator_state(self.state.clone())
            .await
            .unwrap();
    }

    /// Clear the round dictionaries.
    async fn clear_redis_state(&self) {
        self.store
            .clone()
            .connection()
            .await
            .flushdb()
            .await
            .unwrap();
    }

    pub async fn run(&mut self) {
        loop {
            self.proceed_sum_phase().await;
            self.proceed_update_phase().await;
            self.proceed_sum2_phase().await;
            self.proceed_idle_phase().await;
        }
    }

    // Freeze the sum dictionary.
    async fn freeze_sum_dict(&mut self) {
        let sum_pks = self
            .store
            .clone()
            .connection()
            .await
            .get_sum_pks()
            .await
            .unwrap();
        self.sum_phase_cache = Some(Arc::new(SumPhaseCache(sum_pks)));
    }

    // Freeze the mask dictionary.
    async fn freeze_mask_dict(&self) -> Result<Mask, RoundFailed> {
        let mask_dict: Vec<(Mask, usize)> = self
            .store
            .clone()
            .connection()
            .await
            .get_best_masks()
            .await
            .unwrap();

        if mask_dict.is_empty() {
            Err(RoundFailed::NoMask)
        } else {
            let (mask, _) = mask_dict.into_iter().fold(
                (None, 0_usize),
                |(unique_mask, unique_count), (mask, count)| match unique_count.cmp(&count) {
                    Ordering::Less => (Some(mask), count),
                    Ordering::Greater => (unique_mask, unique_count),
                    Ordering::Equal => (None, unique_count),
                },
            );
            mask.ok_or(RoundFailed::AmbiguousMasks)
        }
    }
}

// Counter to count how many messages were processed successfully.
// The MsgCounter is a future that is resolved when `min` messages have been received.
struct MsgCounter {
    min: usize,
    current: usize,
    success_rx: UnboundedReceiver<()>,
}

impl MsgCounter {
    fn new(min: usize) -> (UnboundedSender<()>, Self) {
        let (success_tx, success_rx) = unbounded_channel();
        (
            success_tx,
            Self {
                min,
                current: 0,
                success_rx,
            },
        )
    }
}

impl Future for MsgCounter {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let pin = self.get_mut();

        if let Poll::Ready(_) = pin.success_rx.poll_next_unpin(cx) {
            pin.current += 1;
        }

        if pin.current == pin.min {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

struct MsgHandler {
    success_tx: UnboundedSender<()>,
    limit_msg_processing: Arc<Semaphore>,
}

impl MsgHandler {
    pub async fn run<T>(&self, task: T)
    where
        T: Future<Output = Result<(), PetError>> + Send + 'static,
        T::Output: Send + 'static,
    {
        match task.await {
            Ok(_) => {
                let _ = self.success_tx.send(());
            }
            Err(_) => self.limit_msg_processing.add_permits(1),
        };
    }
    /// Validate and handle a sum message.
    async fn handle_sum_message(
        redis: Connection,
        validation_cache: Arc<ValidationCache>,
        bytes: Vec<u8>,
    ) -> Result<(), PetError> {
        let msg = SumMessage::open(&bytes[..], &validation_cache.pk, &validation_cache.sk)?;
        msg.certificate().validate()?;
        MsgHandler::validate_sum_task(validation_cache, msg.sum_signature(), msg.pk())?;
        MsgHandler::add_sum_participant(redis, msg.pk(), msg.ephm_pk()).await
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
    ) -> Result<(), PetError> {
        let msg = UpdateMessage::open(&bytes[..], &validation_cache.pk, &validation_cache.sk)?;
        msg.certificate().validate()?;
        MsgHandler::validate_update_task(
            validation_cache,
            msg.sum_signature(),
            msg.update_signature(),
            msg.pk(),
        )?;
        MsgHandler::add_local_seed_dict(redis, sum_cache, msg.pk(), msg.local_seed_dict()).await
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
    ) -> Result<(), PetError> {
        let msg = Sum2Message::open(&bytes[..], &validation_cache.pk, &validation_cache.sk)?;
        msg.certificate().validate()?;
        MsgHandler::validate_sum_task(validation_cache, msg.sum_signature(), msg.pk())?;
        MsgHandler::add_mask(redis, msg.pk(), msg.mask()).await
    }

    /// Add a mask to the mask dictionary. Fails if the sum participant didn't register in the sum
    /// phase or it is a repetition.
    async fn add_mask(
        redis: RedisStore,
        pk: &SumParticipantPublicKey,
        mask: &Mask,
    ) -> Result<(), PetError> {
        match redis
            .clone()
            .connection()
            .await
            .remove_sum_dict_entry(*pk)
            .await
        {
            // field was deleted
            Ok(1) => (),
            // field does not exist or redis err
            Ok(_) | Err(_) => return Err(PetError::InvalidMessage),
        }

        redis
            .connection()
            .await
            .incr_mask_count(mask.clone())
            .await
            .map_err(|_| PetError::InvalidMessage)
    }
}

#[derive(Clone)]
/// A cache that contains the sum_pk of the current sum phase.
/// The cache is used to validate update messages.
struct SumPhaseCache(HashSet<SumParticipantPublicKey>);

impl SumPhaseCache {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn sum_pks(&self) -> &HashSet<SumParticipantPublicKey> {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// The local state of the coordinator
pub struct CoordinatorState {
    // credentials
    pk: CoordinatorPublicKey,
    sk: CoordinatorSecretKey,

    // round parameters
    sum: f64,
    update: f64,
    seed: RoundSeed,
    min_sum: usize,
    min_update: usize,
    phase: Phase,
}

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
        }
    }
}

#[derive(Clone)]
/// A cache that contains all the values ​​necessary to validate messages.
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

#[cfg(test)]
mod tests {
    use std::iter;

    use num::{bigint::BigUint, traits::identities::Zero};

    use super::*;
    use crate::{
        crypto::*,
        mask::{
            config::{BoundType, DataType, GroupType, MaskConfigs, ModelType},
            seed::MaskSeed,
        },
    };

    #[tokio::test]
    #[ignore]
    async fn test_validate_sum_task() {
        let store = RedisStore::new("redis://127.0.0.1/", 10).await.unwrap();

        let (msg_tx, mut coord) = Coordinator::new(store).await.unwrap();
        coord.state.sum = 0.5_f64;
        coord.state.update = 0.5_f64;
        coord.state.seed = RoundSeed::from_slice_unchecked(&[
            229, 16, 164, 40, 138, 161, 23, 161, 175, 102, 13, 103, 229, 229, 163, 56, 184, 250,
            190, 44, 91, 69, 246, 222, 64, 101, 139, 22, 126, 6, 103, 238,
        ]);

        // eligible sum signature
        let sum_signature = Signature::from_slice_unchecked(&[
            216, 122, 81, 56, 190, 176, 44, 37, 167, 89, 45, 93, 82, 92, 147, 208, 158, 65, 145,
            253, 121, 35, 80, 38, 4, 37, 65, 244, 185, 101, 59, 124, 21, 22, 184, 234, 226, 78,
            255, 85, 112, 206, 76, 140, 216, 39, 172, 76, 0, 172, 239, 189, 106, 64, 137, 185, 123,
            132, 115, 14, 160, 116, 82, 7,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            76, 128, 23, 65, 195, 57, 190, 223, 67, 224, 102, 139, 140, 90, 67, 160, 106, 181, 7,
            196, 245, 56, 193, 51, 15, 212, 9, 153, 61, 152, 173, 165,
        ]);
        assert_eq!(coord.validate_sum_task(&sum_signature, &pk).unwrap(), ());

        // ineligible sum signature
        let sum_signature = Signature::from_slice_unchecked(&[
            75, 17, 216, 121, 214, 15, 222, 250, 0, 172, 158, 190, 201, 132, 251, 15, 149, 4, 127,
            110, 214, 208, 17, 93, 236, 103, 199, 193, 74, 224, 243, 79, 217, 237, 184, 104, 126,
            203, 18, 189, 248, 237, 116, 163, 42, 32, 236, 96, 181, 151, 144, 252, 211, 56, 141,
            98, 108, 248, 231, 248, 61, 200, 184, 13,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            200, 198, 194, 36, 111, 82, 127, 148, 245, 223, 158, 98, 142, 50, 65, 51, 7, 234, 201,
            148, 45, 56, 85, 65, 75, 128, 178, 175, 101, 93, 241, 162,
        ]);
        assert_eq!(
            coord.validate_sum_task(&sum_signature, &pk).unwrap_err(),
            PetError::InvalidMessage,
        );
    }
}
