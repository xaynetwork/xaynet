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

use self::config::{MaskConfig, MaskConfigs};
use crate::{
    utils::{generate_integer, ratio_as},
    PetError,
    SumParticipantEphemeralPublicKey,
    SumParticipantEphemeralSecretKey,
};

const USIZE_LEN: usize = mem::size_of::<usize>();

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
    pub fn encrypt(&self, pk: &SumParticipantEphemeralPublicKey) -> EncrMaskSeed {
        // safe unwrap: length of slice is guaranteed by constants
        EncrMaskSeed::try_from(pk.encrypt(self.seed.as_ref())).unwrap()
    }

    /// Derive a mask of length `len` from the seed wrt the mask configuration.
    pub fn derive_mask(&self, len: usize, config: &MaskConfig) -> Mask {
        let mut prng = ChaCha20Rng::from_seed(self.seed());
        let integers = iter::repeat_with(|| generate_integer(&mut prng, config.order()))
            .take(len)
            .collect();
        Mask {
            integers,
            config: config.clone(),
        }
    }
}

impl TryFrom<Vec<u8>> for MaskSeed {
    type Error = PetError;

    /// Create a mask seed from bytes. Fails if the length of the input is invalid.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let seed = box_::Seed::from_slice(bytes.as_slice()).ok_or(Self::Error::InvalidMessage)?;
        Ok(Self { seed })
    }
}

impl TryFrom<&[u8]> for MaskSeed {
    type Error = PetError;

    /// Create a mask seed from a slice of bytes. Fails if the length of the input is invalid.
    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        let seed = box_::Seed::from_slice(slice).ok_or(Self::Error::InvalidMessage)?;
        Ok(Self { seed })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// An encrypted mask seed.
pub struct EncrMaskSeed(Vec<u8>);

impl EncrMaskSeed {
    pub const BYTES: usize = sealedbox::SEALBYTES + MaskSeed::BYTES;

    /// Decrypt an encrypted mask seed. Fails if the decryption fails.
    pub fn decrypt(
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
            Err(Self::Error::InvalidMessage)
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
            Err(Self::Error::InvalidMessage)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Model<F: FloatCore> {
    weights: Vec<F>,
}

impl<F: FloatCore> TryFrom<Vec<F>> for Model<F> {
    type Error = PetError;

    /// Create a model from its weights. Fails if the weights are not finite.
    fn try_from(weights: Vec<F>) -> Result<Self, Self::Error> {
        if weights.iter().all(|weight| weight.is_finite()) {
            Ok(Self { weights })
        } else {
            Err(Self::Error::InvalidMessage)
        }
    }
}

impl<F: FloatCore> Model<F> {
    /// Get a reference to the model weights.
    pub fn weights(&'_ self) -> &'_ Vec<F> {
        &self.weights
    }

    /// Mask the model wrt the mask configuration. Enforces the bounds on the scalar and weights.
    pub fn mask(&self, scalar: f64, config: &MaskConfig) -> (MaskSeed, MaskedModel) {
        // safe unwrap: clamped scalar is finite
        let scalar = &Ratio::<BigInt>::from_float(clamp(scalar, 0_f64, 1_f64)).unwrap();
        let negative_bound = &-config.add_shift();
        let positive_bound = config.add_shift();
        let mask_seed = MaskSeed::new();
        let mut prng = ChaCha20Rng::from_seed(mask_seed.seed());
        let integers = self
            .weights
            .iter()
            .map(|weight| {
                // clamp, scale and shift the weight into the non-negative integers
                let integer = (((scalar
                    * clamp(
                        // safe unwrap: `weight` is guaranteed to be finite because of `try_from`
                        &Ratio::<BigInt>::from_float(*weight).unwrap(),
                        negative_bound,
                        positive_bound,
                    ))
                    + config.add_shift())
                    * config.exp_shift())
                .to_integer()
                .to_biguint()
                // safe unwrap: shifted weight is guaranteed to be non-negative
                .unwrap();
                // shift the masked weight into the finite group
                let masked_weight =
                    (integer + generate_integer(&mut prng, config.order())) % config.order();
                masked_weight
            })
            .collect::<Vec<BigUint>>();
        let masked_model = MaskedModel {
            integers,
            config: config.clone(),
        };
        (mask_seed, masked_model)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaskedModel {
    integers: Vec<BigUint>,
    config: MaskConfig,
}

// impl TryFrom<Vec<u8>> for MaskedModel {
//     type Error = PetError;

//     /// Create a masked model from bytes. Fails if deserialization fails.
//     fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
//         Self::deserialize(bytes.as_slice())
//     }
// }

// impl TryFrom<&[u8]> for MaskedModel {
//     type Error = PetError;

//     /// Create a masked model from a slice of bytes. Fails if deserialization fails.
//     fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
//         Self::deserialize(slice)
//     }
// }

impl MaskedModel {
    /// Get a reference to the masked model integers.
    pub fn integers(&'_ self) -> &'_ Vec<BigUint> {
        &self.integers
    }

    /// Unmask the masked model with a mask. Requires the total number of models.
    pub fn unmask<F: FloatCore>(
        &self,
        mask: &Mask,
        no_models: usize,
    ) -> Result<Model<F>, PetError> {
        let scaled_add_shift = self.config.add_shift() * BigInt::from(no_models);
        let weights = self
            .integers
            .iter()
            .zip(mask.integers().iter())
            .map(|(masked_weight, mask)| {
                // unmask the masked weight
                let integer = Ratio::<BigInt>::from(
                    // safe minus: sum is guaranteed to be non-negative
                    ((masked_weight + self.config.order() - mask) % self.config.order())
                        .to_bigint()
                        // safe unwrap: `to_bigint` never fails for `BigUint`s
                        .unwrap(),
                );
                // shift the weight into the reals
                let weight =
                    ratio_as::<F>(&(integer / self.config.exp_shift() - &scaled_add_shift));
                weight
            })
            .collect::<Vec<F>>();
        weights.try_into()
    }

    /// Get the length of the serialized masked model.
    pub fn len(&self) -> usize {
        USIZE_LEN + self.integers.len() * self.config.element_len()
    }

    /// Serialize the masked model into bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let element_len = self.config.element_len();
        let bytes = self
            .integers
            .iter()
            .flat_map(|integer| {
                let mut bytes = integer.to_bytes_le();
                // is this padding ever needed?
                bytes.resize(element_len, 0_u8);
                bytes
            })
            .collect();
        [self.config.serialize(), bytes].concat()
    }

    /// Deserialize the masked model from bytes. Fails if the bytes don't conform to the mask
    /// configuration.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, PetError> {
        if bytes.len() < USIZE_LEN {
            return Err(PetError::InvalidMessage);
        }
        let config = MaskConfig::deserialize(&bytes[..USIZE_LEN])?;
        let element_len = config.element_len();
        if bytes[USIZE_LEN..].len() % element_len != 0 {
            return Err(PetError::InvalidMessage);
        }
        let integers = bytes[USIZE_LEN..]
            .chunks_exact(element_len)
            .map(|chunk| BigUint::from_bytes_le(chunk))
            .collect::<Vec<BigUint>>();
        if integers.iter().all(|integer| integer < config.order()) {
            Ok(Self { integers, config })
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    integers: Vec<BigUint>,
    config: MaskConfig,
}

impl TryFrom<Vec<u8>> for Mask {
    type Error = PetError;

    /// Create a mask from bytes. Fails if deserialization fails.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::deserialize(bytes.as_slice())
    }
}

impl TryFrom<&[u8]> for Mask {
    type Error = PetError;

    /// Create a mask from a slice of bytes. Fails if deserialization fails.
    fn try_from(slice: &[u8]) -> Result<Self, Self::Error> {
        Self::deserialize(slice)
    }
}

impl Mask {
    /// Get a reference to the mask integers.
    pub fn integers(&'_ self) -> &'_ Vec<BigUint> {
        &self.integers
    }

    /// Get the length of the serialized masked model.
    pub fn len(&self) -> usize {
        USIZE_LEN + self.integers.len() * self.config.element_len()
    }

    /// Serialize the mask into bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let element_len = self.config.element_len();
        let bytes = self
            .integers
            .iter()
            .flat_map(|integer| {
                let mut bytes = integer.to_bytes_le();
                // is this padding ever needed?
                bytes.resize(element_len, 0_u8);
                bytes
            })
            .collect();
        [self.config.serialize(), bytes].concat()
    }

    /// Deserialize the mask from bytes. Fails if the bytes don't conform to the mask configuration.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, PetError> {
        if bytes.len() < USIZE_LEN {
            return Err(PetError::InvalidMessage);
        }
        let config = MaskConfig::deserialize(&bytes[..USIZE_LEN])?;
        let element_len = config.element_len();
        if bytes[USIZE_LEN..].len() % element_len != 0 {
            return Err(PetError::InvalidMessage);
        }
        let integers = bytes[USIZE_LEN..]
            .chunks_exact(element_len)
            .map(|chunk| BigUint::from_bytes_le(chunk))
            .collect::<Vec<BigUint>>();
        if integers.iter().all(|integer| integer < config.order()) {
            Ok(Self { integers, config })
        } else {
            Err(PetError::InvalidMessage)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masking() {
        let model = Model::try_from(vec![0_f32, 0.5, -0.5]).unwrap();
        let config = MaskConfigs::PrimeF32M3B0.config();
        let (mask_seed, masked_model) = model.mask(1_f64, &config);
        assert_eq!(model.weights().len(), masked_model.integers().len());
        let mask = mask_seed.derive_mask(3, &config);
        let unmasked_model = masked_model.unmask::<f32>(&mask, 1).unwrap();
        assert_eq!(model.weights(), unmasked_model.weights());
    }

    #[test]
    fn test_maskedmodel_serialization() {
        let model = Model::try_from(vec![0_f32, 0.5, -0.5]).unwrap();
        let config = MaskConfigs::PrimeF32M3B0.config();
        let (_, masked_model) = model.mask(1_f64, &config);
        let len = USIZE_LEN + 3 * 6;
        assert_eq!(masked_model.len(), len);
        let serialized = masked_model.serialize();
        assert_eq!(serialized.len(), len);
        let deserialized = MaskedModel::deserialize(serialized.as_slice()).unwrap();
        assert_eq!(masked_model, deserialized);
    }

    #[test]
    fn test_mask_serialization() {
        let config = MaskConfigs::PrimeF32M3B0.config();
        let mask = MaskSeed::new().derive_mask(10, &config);
        let len = USIZE_LEN + 10 * 6;
        assert_eq!(mask.len(), len);
        let serialized = mask.serialize();
        assert_eq!(serialized.len(), len);
        let deserialized = Mask::deserialize(serialized.as_slice()).unwrap();
        assert_eq!(mask, deserialized);
    }
}
