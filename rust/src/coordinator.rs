use std::{
    cmp::Ordering,
    collections::{HashMap, VecDeque},
    default::Default,
    iter,
};

use derive_more::{AsMut, AsRef};
use sodiumoxide::{
    crypto::{box_, hash::sha256},
    randombytes::randombytes,
};
use thiserror::Error;

use crate::{
    crypto::{generate_encrypt_key_pair, ByteObject, SigningKeySeed},
    mask::{Integers, Mask, MaskIntegers, MaskedModel},
    message::{sum::SumMessage, sum2::Sum2Message, update::UpdateMessage},
    model::Model,
    utils::is_eligible,
    CoordinatorPublicKey,
    CoordinatorSecretKey,
    InitError,
    LocalSeedDict,
    MaskHash,
    ParticipantTaskSignature,
    PetError,
    SeedDict,
    SumDict,
    SumParticipantEphemeralPublicKey,
    SumParticipantPublicKey,
    UpdateParticipantPublicKey,
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

#[derive(Debug, PartialEq, Copy, Clone)]
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

#[derive(AsRef, AsMut, Clone, Debug, PartialEq)]
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
pub struct Coordinator<N> {
    // credentials
    pk: CoordinatorPublicKey, // 32 bytes
    sk: CoordinatorSecretKey, // 32 bytes

    // round parameters
    sum: f64,
    update: f64,
    seed: RoundSeed, // 32 bytes
    min_sum: usize,
    min_update: usize,
    phase: Phase,

    // round dictionaries
    /// Dictionary built during the sum phase.
    sum_dict: SumDict,
    /// Dictionary built during the update phase.
    seed_dict: SeedDict,
    /// Dictionary built during the sum2 phase.
    mask_dict: MaskDict,

    // global models
    model: Option<Model<N>>,
    masked_model: Option<MaskedModel>,

    /// Events emitted by the state machine.
    events: VecDeque<ProtocolEvent>,
}

impl<N> Default for Coordinator<N> {
    fn default() -> Self {
        let pk = CoordinatorPublicKey::zeroed();
        let sk = CoordinatorSecretKey::zeroed();
        let sum = 0.01_f64;
        let update = 0.1_f64;
        let seed = RoundSeed::zeroed();
        let min_sum = 1_usize;
        let min_update = 3_usize;
        let phase = Phase::Idle;
        let sum_dict = SumDict::new();
        let seed_dict = SeedDict::new();
        let mask_dict = MaskDict::new();
        let model = None;
        let masked_model = None;
        let events = VecDeque::new();
        Self {
            pk,
            sk,
            sum,
            update,
            seed,
            min_sum,
            min_update,
            phase,
            sum_dict,
            seed_dict,
            mask_dict,
            model,
            masked_model,
            events,
        }
    }
}

pub trait Coordinators: Sized {
    define_trait_fields!(
        pk, pk_mut, CoordinatorPublicKey;
        sk, sk_mut, CoordinatorSecretKey;
        sum, sum_mut, f64;
        update, update_mut, f64;
        min_sum, min_sum_mut, usize;
        min_update, min_update_mut, usize;
        seed, seed_mut, RoundSeed;
        phase, phase_mut, Phase;
        sum_dict, sum_dict_mut, SumDict;
        seed_dict, seed_dict_mut, SeedDict;
        mask_dict, mask_dict_mut, MaskDict;
        masked_model, masked_model_mut, Option<MaskedModel>;
        events, events_mut, VecDeque<ProtocolEvent>;
    );

    /// Create a coordinator. Fails if there is insufficient system entropy to generate secrets.
    fn new() -> Result<Self, InitError>;

    /// Emit an event.
    fn emit_event(&mut self, event: ProtocolEvent) {
        self.events_mut().push_back(event);
    }

    /// Retrieve the next event.
    fn next_event(&mut self) -> Option<ProtocolEvent> {
        self.events_mut().pop_front()
    }

    /// Validate and handle a sum, update or sum2 message.
    fn handle_message(&mut self, bytes: &[u8]) -> Result<(), PetError> {
        match self.phase() {
            Phase::Idle => Err(PetError::InvalidMessage),
            Phase::Sum => self.handle_sum_message(bytes),
            Phase::Update => self.handle_update_message(bytes),
            Phase::Sum2 => self.handle_sum2_message(bytes),
        }
    }

    /// Validate and handle a sum message.
    fn handle_sum_message(&mut self, bytes: &[u8]) -> Result<(), PetError> {
        let msg = SumMessage::open(bytes, self.pk(), self.sk())?;
        msg.certificate().validate()?;
        self.validate_sum_task(msg.sum_signature(), msg.pk())?;
        self.add_sum_participant(msg.pk(), msg.ephm_pk())?;
        Ok(())
    }

    /// Validate and handle an update message.
    fn handle_update_message(&mut self, bytes: &[u8]) -> Result<(), PetError> {
        let msg = UpdateMessage::open(bytes, self.pk(), self.sk())?;
        msg.certificate().validate()?;
        self.validate_update_task(msg.sum_signature(), msg.update_signature(), msg.pk())?;
        self.aggregate_masked_model(msg.masked_model())?;
        self.add_local_seed_dict(msg.pk(), msg.local_seed_dict())?;
        Ok(())
    }

    /// Validate and handle a sum2 message.
    fn handle_sum2_message(&mut self, bytes: &[u8]) -> Result<(), PetError> {
        let msg = Sum2Message::open(bytes, self.pk(), self.sk())?;
        msg.certificate().validate()?;
        self.validate_sum_task(msg.sum_signature(), msg.pk())?;
        self.add_mask(msg.pk(), msg.mask())?;
        Ok(())
    }

    /// Validate a sum signature and its implied task.
    fn validate_sum_task(
        &self,
        sum_signature: &ParticipantTaskSignature,
        pk: &SumParticipantPublicKey,
    ) -> Result<(), PetError> {
        if pk.verify_detached(sum_signature, &[self.seed().as_slice(), b"sum"].concat())
            && is_eligible(sum_signature, *self.sum())
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Validate an update signature and its implied task.
    fn validate_update_task(
        &self,
        sum_signature: &ParticipantTaskSignature,
        update_signature: &ParticipantTaskSignature,
        pk: &UpdateParticipantPublicKey,
    ) -> Result<(), PetError> {
        if pk.verify_detached(sum_signature, &[self.seed().as_slice(), b"sum"].concat())
            && pk.verify_detached(
                update_signature,
                &[self.seed().as_slice(), b"update"].concat(),
            )
            && !is_eligible(sum_signature, *self.sum())
            && is_eligible(update_signature, *self.update())
        {
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Add a sum participant to the sum dictionary. Fails if it is a repetition.
    fn add_sum_participant(
        &mut self,
        pk: &SumParticipantPublicKey,
        ephm_pk: &SumParticipantEphemeralPublicKey,
    ) -> Result<(), PetError> {
        if !self.sum_dict().contains_key(pk) {
            self.sum_dict_mut().insert(*pk, *ephm_pk);
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Freeze the sum dictionary.
    fn freeze_sum_dict(&mut self) {
        *self.seed_dict_mut() = self
            .sum_dict()
            .keys()
            .map(|pk| (*pk, LocalSeedDict::new()))
            .collect();
    }

    /// Aggregate a local masked model to the global masked model. Fails if the model types don't
    /// conform.
    fn aggregate_masked_model(&mut self, local_masked_model: &MaskedModel) -> Result<(), PetError> {
        *self.masked_model_mut() = if let Some(global_masked_model) = self.masked_model() {
            Some(
                global_masked_model
                    .aggregate(local_masked_model)
                    .or(Err(PetError::InvalidMessage))?,
            )
        } else {
            Some(local_masked_model.clone())
        };
        Ok(())
    }

    /// Add a local seed dictionary to the seed dictionary. Fails if it contains invalid keys or it
    /// is a repetition.
    fn add_local_seed_dict(
        &mut self,
        pk: &UpdateParticipantPublicKey,
        local_seed_dict: &LocalSeedDict,
    ) -> Result<(), PetError> {
        if local_seed_dict.keys().len() == self.sum_dict().keys().len()
            && local_seed_dict
                .keys()
                .all(|pk| self.sum_dict().contains_key(pk))
            && self
                .seed_dict()
                .values()
                .next()
                .map_or(true, |dict| !dict.contains_key(pk))
        {
            for (sum_pk, seed) in local_seed_dict {
                self.seed_dict_mut()
                    .get_mut(sum_pk)
                    .ok_or(PetError::InvalidMessage)?
                    .insert(*pk, seed.clone());
            }
            Ok(())
        } else {
            Err(PetError::InvalidMessage)
        }
    }

    /// Add a mask to the mask dictionary. Fails if the sum participant didn't register in the sum
    /// phase or it is a repetition.
    fn add_mask(&mut self, pk: &SumParticipantPublicKey, mask: &Mask) -> Result<(), PetError> {
        if self.sum_dict_mut().remove(pk).is_none() {
            Err(PetError::InvalidMessage)
        } else if let Some(count) = self.mask_dict_mut().get_mut(mask) {
            *count += 1;
            Ok(())
        } else {
            self.mask_dict_mut().insert(mask.clone(), 1);
            Ok(())
        }
    }

    /// Freeze the mask dictionary.
    fn freeze_mask_dict(&self) -> Result<&Mask, RoundFailed> {
        if self.mask_dict().is_empty() {
            Err(RoundFailed::NoMask)
        } else {
            let (mask, _) = self.mask_dict().iter().fold(
                (None, 0_usize),
                |(unique_mask, unique_count), (mask, count)| match unique_count.cmp(count) {
                    Ordering::Less => (Some(mask), *count),
                    Ordering::Greater => (unique_mask, unique_count),
                    Ordering::Equal => (None, unique_count),
                },
            );
            mask.ok_or(RoundFailed::AmbiguousMasks)
        }
    }

    /// Clear the round dictionaries.
    fn clear_round_dicts(&mut self) {
        self.sum_dict_mut().clear();
        self.sum_dict_mut().shrink_to_fit();
        self.seed_dict_mut().clear();
        self.seed_dict_mut().shrink_to_fit();
        self.mask_dict_mut().clear();
        self.mask_dict_mut().shrink_to_fit();
    }

    /// Generate fresh round credentials.
    fn gen_round_keypair(&mut self) {
        let (pk, sk) = generate_encrypt_key_pair();
        *self.pk_mut() = pk;
        *self.sk_mut() = sk;
    }

    /// Update the round threshold parameters (dummy).
    fn update_round_thresholds(&mut self) {}

    /// Update the seed round parameter.
    fn update_round_seed(&mut self) {
        // safe unwrap: `sk` and `seed` have same number of bytes
        let (_, sk) =
            SigningKeySeed::from_slice_unchecked(self.sk().as_slice()).derive_signing_key_pair();
        let signature = sk.sign_detached(
            &[
                self.seed().as_slice(),
                &self.sum().to_le_bytes(),
                &self.update().to_le_bytes(),
            ]
            .concat(),
        );
        // safe unwrap: length of slice is guaranteed by constants
        *self.seed_mut() =
            RoundSeed::from_slice_unchecked(sha256::hash(signature.as_slice()).as_ref());
    }

    /// Check whether enough sum participants submitted their ephemeral keys to start the update
    /// phase.
    fn has_enough_sums(&self) -> bool {
        self.sum_dict().len() >= *self.min_sum()
    }

    /// Check whether enough update participants submitted their models and seeds to start the sum2
    /// phase.
    fn has_enough_seeds(&self) -> bool {
        self.seed_dict()
            .values()
            .next()
            .map(|dict| dict.len() >= *self.min_update())
            .unwrap_or(false)
    }

    /// Check whether enough sum participants submitted their masks to start the idle phase.
    fn has_enough_masks(&self) -> bool {
        let mask_count = self.mask_dict().values().sum::<usize>();
        mask_count >= *self.min_sum()
    }

    /// End the idle phase and proceed to the sum phase to start the round.
    fn proceed_sum_phase(&mut self) {
        info!("going to sum phase");
        self.gen_round_keypair();
        *self.phase_mut() = Phase::Sum;
        self.emit_event(ProtocolEvent::StartSum(self.round_parameters()));
    }

    /// End the sum phase and proceed to the update phase.
    fn proceed_update_phase(&mut self) {
        info!("going to update phase");
        self.freeze_sum_dict();
        *self.phase_mut() = Phase::Update;
        self.emit_event(ProtocolEvent::StartUpdate(self.sum_dict().clone()));
    }

    /// End the update phase and proceed to the sum2 phase.
    fn proceed_sum2_phase(&mut self) {
        info!("going to sum2 phase");
        *self.phase_mut() = Phase::Sum2;
        self.emit_event(ProtocolEvent::StartSum2(self.seed_dict().clone()));
    }

    /// Prepare the coordinator for a new round and go back to the initial phase.
    fn start_new_round(&mut self) {
        self.clear_round_dicts();
        self.update_round_thresholds();
        self.update_round_seed();
        *self.phase_mut() = Phase::Idle;
    }

    fn round_parameters(&self) -> RoundParameters {
        RoundParameters {
            pk: *self.pk(),
            sum: *self.sum(),
            update: *self.update(),
            seed: self.seed().clone(),
        }
    }
}

pub trait MaskCoordinators<N>: Coordinators {
    define_trait_fields!(model, model_mut, Option<Model<N>>);

    /// Unmask the masked model with a mask.
    fn unmask_model(&self, mask: &Mask) -> Result<Model<N>, RoundFailed>;

    /// Transition to the next phase if the protocol conditions are satisfied.
    fn try_phase_transition(&mut self) {
        match self.phase() {
            Phase::Idle => {
                self.proceed_sum_phase();
                self.try_phase_transition();
            }
            Phase::Sum => {
                if self.has_enough_sums() {
                    self.proceed_update_phase();
                    self.try_phase_transition();
                }
            }
            Phase::Update => {
                if self.has_enough_seeds() {
                    self.proceed_sum2_phase();
                    self.try_phase_transition();
                }
            }
            Phase::Sum2 => {
                if self.has_enough_masks() {
                    self.proceed_idle_phase();
                    self.try_phase_transition();
                }
            }
        }
    }

    /// End the sum2 phase and proceed to the idle phase to end the round.
    fn proceed_idle_phase(&mut self) {
        info!("going to idle phase");
        let outcome = if let Ok(mask) = self.freeze_mask_dict() {
            if let Ok(model) = self.unmask_model(mask) {
                *self.model_mut() = Some(model);
                Some(())
            } else {
                None
            }
        } else {
            None
        };
        self.emit_event(ProtocolEvent::EndRound(outcome));
        self.start_new_round();
    }

    /// Cancel the current round and restart a new one.
    fn reset(&mut self) {
        self.events_mut().clear();
        self.emit_event(ProtocolEvent::EndRound(None));
        self.start_new_round();
        self.try_phase_transition();
    }
}

impl<N> Coordinators for Coordinator<N> {
    derive_trait_fields!(
        pk, pk_mut, CoordinatorPublicKey;
        sk, sk_mut, CoordinatorSecretKey;
        sum, sum_mut, f64;
        update, update_mut, f64;
        min_sum, min_sum_mut, usize;
        min_update, min_update_mut, usize;
        seed, seed_mut, RoundSeed;
        phase, phase_mut, Phase;
        sum_dict, sum_dict_mut, SumDict;
        seed_dict, seed_dict_mut, SeedDict;
        mask_dict, mask_dict_mut, MaskDict;
        masked_model, masked_model_mut, Option<MaskedModel>;
        events, events_mut, VecDeque<ProtocolEvent>;
    );

    /// Create a coordinator. Fails if there is insufficient system entropy to generate secrets.
    fn new() -> Result<Self, InitError> {
        // crucial: init must be called before anything else in this module
        sodiumoxide::init().or(Err(InitError))?;
        let seed = RoundSeed::generate();
        Ok(Self {
            seed,
            ..Default::default()
        })
    }
}

impl MaskCoordinators<f32> for Coordinator<f32> {
    derive_trait_fields!(model, model_mut, Option<Model<f32>>);

    fn unmask_model(&self, mask: &Mask) -> Result<Model<f32>, RoundFailed> {
        let no_models = self.seed_dict.values().next().map_or(0, |dict| dict.len());
        if let Some(masked_model) = self.masked_model() {
            masked_model
                .unmask(mask, no_models)
                .or(Err(RoundFailed::NoModel))
        } else {
            Err(RoundFailed::NoModel)
        }
    }
}

impl MaskCoordinators<f64> for Coordinator<f64> {
    derive_trait_fields!(model, model_mut, Option<Model<f64>>);

    fn unmask_model(&self, mask: &Mask) -> Result<Model<f64>, RoundFailed> {
        let no_models = self.seed_dict.values().next().map_or(0, |dict| dict.len());
        if let Some(masked_model) = self.masked_model() {
            masked_model
                .unmask(mask, no_models)
                .or(Err(RoundFailed::NoModel))
        } else {
            Err(RoundFailed::NoModel)
        }
    }
}

impl MaskCoordinators<i32> for Coordinator<i32> {
    derive_trait_fields!(model, model_mut, Option<Model<i32>>);

    fn unmask_model(&self, mask: &Mask) -> Result<Model<i32>, RoundFailed> {
        let no_models = self.seed_dict.values().next().map_or(0, |dict| dict.len());
        if let Some(masked_model) = self.masked_model() {
            masked_model
                .unmask(mask, no_models)
                .or(Err(RoundFailed::NoModel))
        } else {
            Err(RoundFailed::NoModel)
        }
    }
}

impl MaskCoordinators<i64> for Coordinator<i64> {
    derive_trait_fields!(model, model_mut, Option<Model<i64>>);

    fn unmask_model(&self, mask: &Mask) -> Result<Model<i64>, RoundFailed> {
        let no_models = self.seed_dict.values().next().map_or(0, |dict| dict.len());
        if let Some(masked_model) = self.masked_model() {
            masked_model
                .unmask(mask, no_models)
                .or(Err(RoundFailed::NoModel))
        } else {
            Err(RoundFailed::NoModel)
        }
    }
}

#[derive(Debug, PartialEq)]
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
    use num::{bigint::BigUint, traits::identities::Zero};

    use super::*;
    use crate::{
        crypto::*,
        mask::{
            config::{BoundType, DataType, GroupType, MaskConfigs, ModelType},
            seed::MaskSeed,
        },
    };

    #[test]
    fn test_coordinator() {
        let coord = Coordinator::<f32>::new().unwrap();
        assert_eq!(coord.pk, PublicEncryptKey::zeroed());
        assert_eq!(coord.sk, SecretEncryptKey::zeroed());
        assert!(coord.sum >= 0. && coord.sum <= 1.);
        assert!(coord.update >= 0. && coord.update <= 1.);
        assert_eq!(coord.seed.as_slice().len(), 32);
        assert!(coord.min_sum >= 1);
        assert!(coord.min_update >= 3);
        assert_eq!(coord.phase, Phase::Idle);
        assert_eq!(coord.sum_dict, SumDict::new());
        assert_eq!(coord.seed_dict, SeedDict::new());
        assert_eq!(coord.mask_dict, MaskDict::new());
        assert_eq!(coord.model, None);
        assert_eq!(coord.masked_model, None);
    }

    #[test]
    fn test_validate_sum_task() {
        let mut coord = Coordinator::<f32>::new().unwrap();
        coord.sum = 0.5_f64;
        coord.update = 0.5_f64;
        coord.seed = RoundSeed::from_slice_unchecked(&[
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

    #[test]
    fn test_validate_update_task() {
        let mut coord = Coordinator::<f32>::new().unwrap();
        coord.sum = 0.5_f64;
        coord.update = 0.5_f64;
        coord.seed = RoundSeed::from_slice_unchecked(&[
            229, 16, 164, 40, 138, 161, 23, 161, 175, 102, 13, 103, 229, 229, 163, 56, 184, 250,
            190, 44, 91, 69, 246, 222, 64, 101, 139, 22, 126, 6, 103, 238,
        ]);

        // ineligible sum signature and eligible update signature
        let sum_signature = Signature::from_slice_unchecked(&[
            206, 154, 228, 165, 240, 196, 64, 106, 135, 124, 140, 83, 15, 188, 229, 78, 38, 34,
            254, 241, 7, 23, 44, 147, 6, 195, 158, 227, 250, 159, 60, 214, 42, 103, 145, 69, 121,
            165, 115, 196, 120, 164, 108, 200, 114, 200, 16, 21, 208, 233, 83, 176, 70, 77, 64,
            141, 65, 63, 236, 184, 250, 127, 59, 8,
        ]);
        let update_signature = Signature::from_slice_unchecked(&[
            76, 195, 29, 117, 72, 226, 246, 103, 166, 245, 16, 122, 235, 107, 96, 111, 149, 231,
            216, 62, 1, 206, 139, 127, 208, 254, 118, 43, 0, 193, 54, 40, 2, 144, 240, 162, 240,
            226, 223, 0, 228, 59, 13, 252, 42, 34, 16, 22, 202, 30, 166, 138, 231, 2, 125, 123, 75,
            146, 103, 149, 95, 7, 177, 15,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            220, 150, 230, 193, 226, 222, 50, 73, 44, 227, 70, 25, 58, 237, 34, 184, 151, 253, 127,
            252, 13, 23, 135, 194, 244, 12, 139, 17, 34, 61, 9, 92,
        ]);
        assert_eq!(
            coord
                .validate_update_task(&sum_signature, &update_signature, &pk)
                .unwrap(),
            (),
        );

        // ineligible sum signature and ineligible update signature
        let sum_signature = Signature::from_slice_unchecked(&[
            73, 255, 75, 96, 89, 197, 182, 203, 156, 41, 231, 88, 103, 16, 204, 35, 52, 165, 178,
            159, 33, 199, 112, 59, 203, 58, 243, 229, 190, 226, 168, 96, 146, 49, 79, 147, 224,
            235, 140, 247, 101, 99, 255, 179, 150, 219, 84, 69, 146, 49, 182, 105, 42, 65, 159, 41,
            118, 214, 172, 240, 213, 27, 192, 12,
        ]);
        let update_signature = Signature::from_slice_unchecked(&[
            163, 180, 225, 224, 231, 2, 162, 183, 211, 242, 26, 56, 124, 179, 241, 13, 105, 29,
            240, 251, 89, 126, 147, 229, 138, 68, 118, 206, 102, 193, 209, 79, 219, 109, 87, 59,
            197, 177, 197, 213, 79, 143, 149, 66, 159, 107, 139, 244, 6, 224, 111, 175, 90, 213,
            206, 143, 152, 0, 21, 15, 102, 74, 15, 14,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            109, 181, 253, 91, 247, 2, 201, 224, 161, 207, 128, 48, 16, 201, 86, 14, 193, 204, 49,
            88, 9, 170, 109, 120, 245, 0, 208, 129, 107, 213, 253, 72,
        ]);
        assert_eq!(
            coord
                .validate_update_task(&sum_signature, &update_signature, &pk)
                .unwrap_err(),
            PetError::InvalidMessage,
        );

        // eligible sum signature and eligible update signature
        let sum_signature = Signature::from_slice_unchecked(&[
            22, 28, 85, 58, 83, 51, 179, 43, 142, 58, 15, 113, 125, 191, 145, 179, 22, 216, 183,
            114, 230, 219, 151, 4, 213, 187, 197, 160, 171, 240, 40, 0, 133, 132, 7, 117, 105, 37,
            84, 214, 243, 19, 187, 132, 80, 194, 214, 204, 58, 130, 33, 63, 40, 149, 30, 27, 106,
            122, 254, 106, 161, 61, 176, 5,
        ]);
        let update_signature = Signature::from_slice_unchecked(&[
            7, 50, 23, 176, 28, 214, 185, 141, 131, 236, 166, 140, 232, 21, 223, 88, 16, 98, 202,
            232, 46, 210, 102, 177, 107, 196, 87, 192, 36, 153, 175, 104, 208, 61, 179, 151, 191,
            103, 75, 70, 109, 185, 10, 215, 28, 29, 12, 68, 15, 124, 248, 159, 57, 84, 156, 83,
            189, 233, 8, 184, 197, 21, 51, 1,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            212, 224, 51, 239, 70, 208, 166, 236, 81, 5, 7, 226, 54, 151, 50, 223, 133, 134, 66,
            167, 32, 226, 141, 200, 232, 41, 112, 144, 79, 135, 207, 87,
        ]);
        assert_eq!(
            coord
                .validate_update_task(&sum_signature, &update_signature, &pk)
                .unwrap_err(),
            PetError::InvalidMessage,
        );

        // eligible sum signature and ineligible update signature
        let sum_signature = Signature::from_slice_unchecked(&[
            176, 1, 85, 13, 43, 110, 122, 206, 186, 247, 44, 215, 154, 222, 34, 34, 173, 139, 166,
            42, 239, 160, 167, 126, 72, 234, 114, 1, 236, 10, 210, 155, 170, 33, 138, 129, 178, 56,
            154, 228, 84, 174, 187, 242, 3, 224, 143, 102, 134, 47, 49, 33, 103, 107, 147, 51, 36,
            143, 215, 134, 213, 162, 255, 5,
        ]);
        let update_signature = Signature::from_slice_unchecked(&[
            39, 29, 201, 153, 218, 79, 161, 208, 151, 222, 220, 95, 118, 156, 17, 49, 35, 125, 243,
            214, 83, 240, 196, 168, 166, 225, 86, 103, 140, 237, 252, 196, 11, 5, 85, 18, 126, 210,
            82, 14, 88, 198, 114, 39, 239, 226, 243, 28, 48, 22, 39, 19, 244, 103, 13, 92, 216,
            251, 155, 154, 180, 114, 158, 13,
        ]);
        let pk = PublicSigningKey::from_slice_unchecked(&[
            251, 251, 252, 131, 93, 84, 116, 191, 88, 135, 45, 43, 201, 66, 7, 236, 40, 74, 17, 11,
            33, 126, 224, 127, 77, 232, 59, 34, 120, 174, 137, 2,
        ]);
        assert_eq!(
            coord
                .validate_update_task(&sum_signature, &update_signature, &pk)
                .unwrap_err(),
            PetError::InvalidMessage,
        );
    }

    fn auxiliary_sum(min_sum: usize) -> SumDict {
        iter::repeat_with(|| {
            (
                PublicSigningKey::from_slice_unchecked(&randombytes(32)),
                PublicEncryptKey::from_slice_unchecked(&randombytes(32)),
            )
        })
        .take(min_sum)
        .collect()
    }

    #[test]
    fn test_sum_dict() {
        let mut coord = Coordinator::<f32>::new().unwrap();
        coord.min_sum = 3;
        coord.min_update = 3;
        coord.try_phase_transition(); // start the sum phase
        assert_eq!(
            coord.next_event().unwrap(),
            ProtocolEvent::StartSum(RoundParameters {
                sum: 0.01,
                update: 0.1,
                seed: coord.seed.clone(),
                pk: coord.pk,
            })
        );
        assert!(coord.next_event().is_none());
        assert_eq!(coord.phase, Phase::Sum);
        assert!(coord.sum_dict.is_empty());

        // Artifically add just enough sum participants
        let sum_dict = auxiliary_sum(coord.min_sum);
        for (pk, ephm_pk) in sum_dict.iter() {
            assert!(!coord.has_enough_sums());
            coord.add_sum_participant(pk, ephm_pk).unwrap();
        }
        assert_eq!(coord.sum_dict, sum_dict);
        assert!(coord.seed_dict.is_empty());
        assert!(coord.has_enough_sums());

        coord.try_phase_transition(); // finish the sum phase
        assert_eq!(
            coord.next_event().unwrap(),
            ProtocolEvent::StartUpdate(sum_dict.clone())
        );
        assert!(coord.next_event().is_none());
        assert_eq!(coord.phase, Phase::Update);
        assert_eq!(
            coord.seed_dict,
            sum_dict
                .iter()
                .map(|(pk, _)| (*pk, LocalSeedDict::new()))
                .collect(),
        );
    }

    fn generate_update(sum_dict: &SumDict) -> (UpdateParticipantPublicKey, LocalSeedDict) {
        let seed = MaskSeed::generate();
        let pk = PublicSigningKey::from_slice_unchecked(&randombytes(32));
        let local_seed_dict = sum_dict
            .iter()
            .map(|(sum_pk, sum_ephm_pk)| (*sum_pk, seed.encrypt(sum_ephm_pk)))
            .collect::<LocalSeedDict>();
        (pk, local_seed_dict)
    }

    fn auxiliary_update(
        min_sum: usize,
        min_update: usize,
    ) -> (
        SumDict,
        Vec<(UpdateParticipantPublicKey, LocalSeedDict)>,
        SeedDict,
    ) {
        let sum_dict = auxiliary_sum(min_sum);
        let updates = iter::repeat_with(|| generate_update(&sum_dict))
            .take(min_update)
            .collect::<Vec<(UpdateParticipantPublicKey, LocalSeedDict)>>();
        let mut seed_dict = SeedDict::new();
        for sum_pk in sum_dict.keys() {
            // Dictionary of all the encrypted seeds for that participant
            let sum_participant_seeds = updates
                .iter()
                .map(|(upd_pk, local_seed_dict)| {
                    (*upd_pk, local_seed_dict.get(sum_pk).unwrap().clone())
                })
                .collect();
            seed_dict.insert(*sum_pk, sum_participant_seeds);
        }
        (sum_dict, updates, seed_dict)
    }

    #[test]
    fn test_seed_dict() {
        let mut coord = Coordinator::<f32>::new().unwrap();
        coord.min_sum = 3;
        coord.min_update = 3;
        coord.try_phase_transition(); // start the sum phase
        assert!(coord.next_event().is_some());
        assert!(coord.next_event().is_none());

        // artificially populate the sum dictionary
        let (sum_dict, updates, seed_dict) = auxiliary_update(coord.min_sum, coord.min_update);
        coord.sum_dict = sum_dict;

        coord.try_phase_transition(); // start the update phase
        assert!(coord.next_event().is_some());
        assert!(coord.next_event().is_none());
        assert_eq!(coord.phase, Phase::Update);
        assert!(!coord.has_enough_seeds());

        // simulate update participants sending their seeds dictionary
        for (pk, local_seed_dict) in updates.iter() {
            assert!(!coord.has_enough_seeds());
            coord.add_local_seed_dict(pk, local_seed_dict).unwrap();
        }
        assert_eq!(coord.seed_dict, seed_dict);
        assert!(coord.has_enough_seeds());

        coord.try_phase_transition(); // finish the update phase
        assert_eq!(
            coord.next_event().unwrap(),
            ProtocolEvent::StartSum2(seed_dict)
        );
        assert!(coord.next_event().is_none());
        assert_eq!(coord.phase, Phase::Sum2);
    }

    fn auxiliary_mask(min_sum: usize) -> (Vec<Mask>, MaskDict) {
        let config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();
        let masks = [
            vec![MaskSeed::generate().derive_mask(10, &config); min_sum - 1],
            vec![MaskSeed::generate().derive_mask(10, &config); 1],
        ]
        .concat();
        let mask_dict = [
            (masks[0].clone(), min_sum - 1),
            (masks[min_sum - 1].clone(), 1),
        ]
        .iter()
        .cloned()
        .collect::<MaskDict>();
        (masks, mask_dict)
    }

    #[test]
    fn test_mask_dict() {
        let mut coord = Coordinator::<f32>::new().unwrap();
        coord.min_sum = 3;
        coord.min_update = 3;
        coord.phase = Phase::Sum2;

        // Pretend we received enough masks
        let sum_dict = auxiliary_sum(coord.min_sum);
        coord.sum_dict = sum_dict.clone();
        let (masks, mask_dict) = auxiliary_mask(coord.min_sum);
        for (pk, mask) in sum_dict.keys().zip(masks.iter()) {
            coord.add_mask(pk, mask).unwrap();
        }
        assert_eq!(coord.mask_dict, mask_dict);
        assert!(coord.has_enough_masks());
        assert_eq!(coord.freeze_mask_dict().unwrap(), &masks[0]);
    }

    #[test]
    fn test_mask_dict_fail() {
        let mut coord = Coordinator::<f32>::new().unwrap();
        coord.min_sum = 3;
        coord.min_update = 3;
        coord.phase = Phase::Sum2;

        let config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();
        coord.mask_dict = iter::repeat_with(|| (MaskSeed::generate().derive_mask(10, &config), 1))
            .take(coord.min_sum)
            .collect::<MaskDict>();
        assert_eq!(
            coord.freeze_mask_dict().unwrap_err(),
            RoundFailed::AmbiguousMasks,
        );
    }

    #[test]
    fn test_clear_round_dicts() {
        let mut coord = Coordinator::<f32>::new().unwrap();
        coord.clear_round_dicts();
        assert!(coord.sum_dict.is_empty());
        assert!(coord.seed_dict.is_empty());
        assert!(coord.mask_dict.is_empty());
    }

    #[test]
    fn test_gen_round_keypair() {
        let mut coord = Coordinator::<f32>::new().unwrap();
        coord.gen_round_keypair();
        assert_eq!(coord.pk, coord.sk.public_key());
        assert_eq!(coord.sk.as_slice().len(), 32);
    }

    #[test]
    fn test_update_round_seed() {
        let mut coord = Coordinator::<f32>::new().unwrap();
        coord.seed = RoundSeed::from_slice_unchecked(&[
            229, 16, 164, 40, 138, 161, 23, 161, 175, 102, 13, 103, 229, 229, 163, 56, 184, 250,
            190, 44, 91, 69, 246, 222, 64, 101, 139, 22, 126, 6, 103, 238,
        ]);
        coord.sk = SecretEncryptKey::from_slice_unchecked(&[
            39, 177, 238, 71, 112, 48, 60, 73, 246, 28, 143, 222, 211, 114, 29, 34, 174, 28, 77,
            51, 146, 27, 155, 224, 20, 169, 254, 164, 231, 141, 190, 31,
        ]);
        coord.update_round_seed();
        assert_eq!(
            coord.seed,
            RoundSeed::from_slice_unchecked(&[
                90, 35, 97, 78, 70, 149, 40, 131, 149, 211, 30, 236, 194, 175, 156, 76, 85, 43,
                138, 159, 180, 166, 25, 205, 156, 176, 3, 203, 27, 128, 231, 38,
            ]),
        );
    }

    #[test]
    fn test_transitions() {
        let mut coord = Coordinator::<f32>::new().unwrap();
        coord.min_sum = 3;
        coord.min_update = 3;

        let (sum_dict, _, seed_dict) = auxiliary_update(coord.min_sum, coord.min_update);
        let (_, mask_dict) = auxiliary_mask(coord.min_sum);
        assert_eq!(coord.phase, Phase::Idle);
        assert!(coord.next_event().is_none());

        coord.try_phase_transition();
        assert_eq!(
            coord.next_event().unwrap(),
            ProtocolEvent::StartSum(RoundParameters {
                sum: 0.01,
                update: 0.1,
                seed: coord.seed.clone(),
                pk: coord.pk,
            })
        );
        assert!(coord.next_event().is_none());
        assert_eq!(coord.phase, Phase::Sum);
        assert_ne!(coord.pk, PublicEncryptKey::zeroed());
        assert_ne!(coord.sk, SecretEncryptKey::zeroed());

        coord.try_phase_transition();
        // We didn't add any participant so the state should remain
        // unchanged
        assert!(coord.next_event().is_none());
        assert_eq!(coord.phase, Phase::Sum);

        // Pretend we have enough participants, and transition
        // again. This time, the state should change.
        coord.sum_dict = sum_dict.clone();
        coord.try_phase_transition();
        assert_eq!(
            coord.next_event().unwrap(),
            ProtocolEvent::StartUpdate(sum_dict)
        );
        assert!(coord.next_event().is_none());
        assert_eq!(coord.phase, Phase::Update);

        // We didn't add any update so the state should remain
        // unchanged
        coord.try_phase_transition();
        assert!(coord.next_event().is_none());
        assert_eq!(coord.phase, Phase::Update);

        // Pretend we received enough updates and transition. This
        // time the state should change.
        coord.seed_dict = seed_dict.clone();
        coord.try_phase_transition();
        assert_eq!(
            coord.next_event().unwrap(),
            ProtocolEvent::StartSum2(seed_dict)
        );
        assert!(coord.next_event().is_none());
        assert_eq!(coord.phase, Phase::Sum2);

        // We didn't add any mask so the state should remain unchanged
        coord.try_phase_transition();
        assert!(coord.next_event().is_none());
        assert_eq!(coord.phase, Phase::Sum2);

        // Pretend we received enough masks and transition. This time
        // the state should change and we should restart a round
        let integers = vec![BigUint::zero(); 10];
        let config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();
        coord.masked_model = Some(MaskedModel::from_parts(integers, config).unwrap());
        coord.mask_dict = mask_dict;
        let seed = coord.seed.clone();
        coord.try_phase_transition();
        assert_eq!(
            coord.next_event().unwrap(),
            ProtocolEvent::EndRound(Some(()))
        );
        assert_eq!(
            coord.next_event().unwrap(),
            ProtocolEvent::StartSum(RoundParameters {
                sum: 0.01,
                update: 0.1,
                seed: coord.seed.clone(),
                pk: coord.pk,
            })
        );
        assert_eq!(coord.phase, Phase::Sum);
        assert!(coord.next_event().is_none());
        assert!(coord.sum_dict.is_empty());
        assert!(coord.seed_dict.is_empty());
        assert!(coord.mask_dict.is_empty());
        assert_ne!(coord.seed, seed);
    }
}
