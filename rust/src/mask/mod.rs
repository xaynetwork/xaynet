pub mod config;

use std::{
    convert::{TryFrom, TryInto},
    default::Default,
    iter,
    mem,
};

use num::{
    bigint::{BigInt, BigUint, ToBigInt},
    rational::{BigRational, Ratio},
    traits::{clamp, float::FloatCore, int::PrimInt, pow::Pow, Num},
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sodiumoxide::{
    crypto::{box_, sealedbox},
    randombytes::{randombytes, randombytes_into},
};

use self::config::MaskConfig;
use crate::{
    utils::{gen_integer, ratio_as},
    PetError,
    SumParticipantEphemeralPublicKey,
    SumParticipantEphemeralSecretKey,
};

#[derive(Clone, Debug, PartialEq)]
pub struct MaskSeed {
    seed: box_::Seed,
}

impl MaskSeed {
    pub const BYTES: usize = box_::SEEDBYTES;

    #[allow(clippy::new_without_default)]
    /// Create a mask seed.
    pub fn new() -> Self {
        // safe unwrap: length of slice is guaranteed by constants
        let seed = box_::Seed::from_slice(&randombytes(Self::BYTES)).unwrap();
        Self { seed }
    }

    pub fn seed(&self) -> [u8; Self::BYTES] {
        self.seed.0
    }

    /// Encrypt a mask seed.
    pub fn seal(&self, pk: &SumParticipantEphemeralPublicKey) -> EncrMaskSeed {
        // safe unwrap: length of slice is guaranteed by constants
        EncrMaskSeed::try_from(pk.encrypt(self.seed.as_ref())).unwrap()
    }
}

impl TryFrom<Vec<u8>> for MaskSeed {
    type Error = PetError;

    /// Create a mask seed from bytes. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let seed = box_::Seed::from_slice(bytes.as_slice()).ok_or(PetError::InvalidMessage)?;
        Ok(Self { seed })
    }
}

impl TryFrom<&[u8]> for MaskSeed {
    type Error = PetError;

    /// Create a mask seed from a slice of bytes. Fails if the length of the input is invalid.
    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        let seed = box_::Seed::from_slice(slice).ok_or(PetError::InvalidMessage)?;
        Ok(Self { seed })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// An encrypted mask seed.
pub struct EncrMaskSeed(Vec<u8>);

impl EncrMaskSeed {
    pub const BYTES: usize = sealedbox::SEALBYTES + MaskSeed::BYTES;

    /// Decrypt an encrypted mask seed. Fails if the decryption fails.
    pub fn open(
        &self,
        pk: &SumParticipantEphemeralPublicKey,
        sk: &SumParticipantEphemeralSecretKey,
    ) -> Result<MaskSeed, PetError> {
        MaskSeed::try_from(
            sk.decrypt(self.as_ref(), pk)
                .or(Err(PetError::InvalidMessage))?,
        )
    }
}

impl AsRef<[u8]> for EncrMaskSeed {
    /// Get a reference to the encrypted mask seed.
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl TryFrom<Vec<u8>> for EncrMaskSeed {
    type Error = PetError;

    /// Create an encrypted mask seed from bytes. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.len() == Self::BYTES {
            Ok(Self(bytes))
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

impl TryFrom<&[u8]> for EncrMaskSeed {
    type Error = PetError;

    /// Create an encrypted mask seed from a slice of bytes. Fails if the length of the input is
    /// invalid.
    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() == Self::BYTES {
            Ok(Self(slice.to_vec()))
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

pub struct Model<F: FloatCore> {
    weights: Vec<F>,
}

impl<F: FloatCore> From<Vec<F>> for Model<F> {
    /// Create a model from its weights.
    fn from(weights: Vec<F>) -> Self {
        Self { weights }
    }
}

impl<F: FloatCore> Model<F> {
    /// Get a reference to the model weights.
    pub fn weights(&self) -> &Vec<F> {
        &self.weights
    }

    /// Mask the model wrt the mask configuration.
    pub fn mask(&self, scalar: F, config: MaskConfig<F>) -> (MaskSeed, MaskedModel) {
        let mask_seed = MaskSeed::new();
        let mut prng = ChaCha20Rng::from_seed(mask_seed.seed());
        // safe unwrap: `add_shift` is guaranteed to be finite
        let add_shift = Ratio::<BigInt>::from_float(config.add_shift()).unwrap();
        let exp_shift = BigInt::from(10_usize).pow(config.exp_shift());
        let masked_weights = self
            .weights
            .iter()
            .map(|weight| {
                // clamp, scale and shift the weight into the non-negative integers
                let integer = ((Ratio::<BigInt>::from_float(
                    scalar * clamp(*weight, -config.add_shift(), config.add_shift()),
                )
                // safe unwrap: scaled weight is guaranteed to be finite
                .unwrap()
                    + &add_shift)
                    * &exp_shift)
                    .to_integer()
                    .to_biguint()
                    // safe unwrap: shifted weight is guaranteed to be non-negative
                    .unwrap();
                // shift the masked weight into the finite group
                let masked_weight =
                    (integer + gen_integer(&mut prng, config.order())) % config.order();
                masked_weight
            })
            .collect::<Vec<BigUint>>();
        (mask_seed, MaskedModel { masked_weights })
    }
}

pub trait BigUintSerde {
    fn integers(&'_ self) -> &'_ Vec<BigUint>;

    /// Get the length of the serialized integers.
    fn len(&self) -> usize {
        self.serialize().len() // todo
    }

    /// Serialize the integers into bytes.
    fn serialize(&self) -> Vec<u8> {
        self.integers()
            .iter()
            .flat_map(|masked_weight| {
                let bytes = masked_weight.to_bytes_le();
                [bytes.len().to_le_bytes().to_vec(), bytes].concat()
            })
            .collect()
    }

    /// Deserialize the integers from bytes.
    fn deserialize(bytes: &[u8]) -> Result<Vec<BigUint>, PetError> {
        let usize_len = mem::size_of::<usize>();
        let mut idx = 0_usize;
        let mut biguint_len: usize;
        let mut integers = Vec::<BigUint>::new();
        while idx < bytes.len() {
            if idx + usize_len <= bytes.len() {
                // safe unwrap: length of slice is guaranteed by constants
                biguint_len = usize::from_le_bytes(bytes[idx..idx + usize_len].try_into().unwrap());
                idx += usize_len;
            } else {
                return Err(PetError::InvalidMessage);
            }
            if idx + biguint_len <= bytes.len() {
                integers.push(BigUint::from_bytes_le(&bytes[idx..idx + biguint_len]));
                idx += biguint_len;
            } else {
                return Err(PetError::InvalidMessage);
            }
        }
        Ok(integers)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaskedModel {
    masked_weights: Vec<BigUint>,
}

impl TryFrom<&[u8]> for MaskedModel {
    type Error = PetError;

    /// Create a masked model from bytes. Fails if deserialization fails.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let masked_weights = Self::deserialize(bytes)?;
        Ok(Self { masked_weights })
    }
}

impl BigUintSerde for MaskedModel {
    fn integers(&'_ self) -> &'_ Vec<BigUint> {
        &self.masked_weights
    }
}

impl MaskedModel {
    /// Unmask the masked model wrt the mask configuration.
    pub fn unmask<F: FloatCore>(
        &self,
        mask: Vec<BigUint>,
        no_models: usize,
        config: MaskConfig<F>,
    ) -> Model<F> {
        let add_shift =
            Ratio::<BigInt>::from_float(config.add_shift()).unwrap() * BigInt::from(no_models);
        let exp_shift = BigInt::from(10_usize).pow(config.exp_shift());
        let weights = self
            .masked_weights
            .iter()
            .zip(mask.iter())
            .map(|(masked_weight, mask)| {
                // unmask the masked weight
                let integer = Ratio::<BigInt>::from(
                    ((masked_weight + config.order() - mask) % config.order())
                        .to_bigint()
                        // safe unwrap: `to_bigint` never fails for `BigUint`s
                        .unwrap(),
                );
                // shift the weight into the reals
                let weight = ratio_as::<F>(integer / &exp_shift - &add_shift);
                weight
            })
            .collect::<Vec<F>>();
        weights.into()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Mask {
    integers: Vec<BigUint>,
}

impl TryFrom<&[u8]> for Mask {
    type Error = PetError;

    /// Create a mask from bytes. Fails if deserialization fails.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let integers = Self::deserialize(bytes)?;
        Ok(Self { integers })
    }
}

impl BigUintSerde for Mask {
    fn integers(&'_ self) -> &'_ Vec<BigUint> {
        &self.integers
    }
}

impl Mask {
    pub fn generate<F: FloatCore>(seed: MaskSeed, config: MaskConfig<F>, len: usize) -> Self {
        let mut prng = ChaCha20Rng::from_seed(seed.seed());
        let integers = iter::repeat_with(|| gen_integer(&mut prng, config.order()))
            .take(len)
            .collect();
        Self { integers }
    }
}

#[test]
fn test() {
    use self::config::MaskConfigs;

    let model = Model::from(vec![0_f32, 0.5, -0.5]);
    let scalar = 0.5_f32;
    let config = MaskConfigs::PrimeF32M3B0.config();
    let (mask_seed, masked_model) = model.mask(scalar, config);
    println!("{:?}", mask_seed.seed());
    println!("{:?}", masked_model.serialize());
}
