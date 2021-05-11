//! Scalar representation and conversion.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]: crate::mask

use crate::mask::{
    model::{ratio_to_float, PrimitiveType},
    PrimitiveCastError,
};
use derive_more::{From, Into};
use num::{
    clamp,
    rational::Ratio,
    traits::{float::FloatCore, ToPrimitive},
    BigInt,
    BigUint,
    One,
    Unsigned,
    Zero,
};
use serde::{Deserialize, Serialize};
use std::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Hash, From, Into, Serialize, Deserialize)]
/// A numerical representation of a machine learning scalar.
pub struct Scalar(Ratio<BigUint>);

impl From<Scalar> for Ratio<BigInt> {
    fn from(scalar: Scalar) -> Self {
        let (numer, denom) = scalar.0.into();
        Ratio::new(numer.into(), denom.into())
    }
}

impl TryFrom<Ratio<BigInt>> for Scalar {
    type Error = <BigUint as TryFrom<BigInt>>::Error;

    fn try_from(ratio: Ratio<BigInt>) -> Result<Self, Self::Error> {
        let (numer, denom) = ratio.into();
        Ok(Self(Ratio::new(numer.try_into()?, denom.try_into()?)))
    }
}

impl Scalar {
    /// Constructs a new `Scalar` from the given numerator and denominator.
    pub fn new<U>(numer: U, denom: U) -> Self
    where
        U: Unsigned + Into<BigUint>,
    {
        Self(Ratio::new(numer.into(), denom.into()))
    }

    /// Constructs a `Scalar` representing the given integer.
    pub fn from_integer<U>(u: U) -> Self
    where
        U: Unsigned + Into<BigUint>,
    {
        Self(Ratio::from_integer(u.into()))
    }

    /// Constructs a `Scalar` of unit value.
    pub fn unit() -> Self {
        Self(Ratio::one())
    }

    /// Convenience method for conversion to a non-negative ratio of `BigInt`.
    pub(crate) fn to_ratio(&self) -> Ratio<BigInt> {
        self.clone().into()
    }

    /// Constructs a `Scalar` from a primitive floating point value, clamped where necessary.
    ///
    /// Maps positive infinity to max of the primitive data type, negatives and NaN to zero.
    pub(crate) fn from_float_bounded<F: FloatCore>(f: F) -> Self {
        if f.is_nan() {
            Self(Ratio::zero())
        } else {
            let finite_f = clamp(f, F::zero(), F::max_value());
            // safe unwrap: clamped weight is guaranteed to be finite
            let r = Ratio::from_float(finite_f).unwrap();
            // safe unwrap: bounded non-negative ratio r
            r.try_into().unwrap()
        }
    }
}

#[derive(Error, Debug)]
#[error("Could not convert weight {weight} to primitive type {target}")]
/// Errors related to scalar conversion into primitives.
pub struct ScalarCastError {
    weight: Ratio<BigUint>,
    target: PrimitiveType,
}

/// An interface for conversion into a primitive value.
///
/// This trait is used to convert a [`Scalar`], which has its own internal
/// representation, into a primitive type ([`f32`], [`f64`], [`i32`], [`i64`]).
/// The opposite trait is [`FromPrimitive`].
pub trait IntoPrimitive<P>: Sized {
    /// Consumes into a converted primitive value.
    ///
    /// # Errors
    /// Returns an error if the conversion fails.
    fn into_primitive(self) -> Result<P, ScalarCastError>;

    /// Converts to a primitive value.
    ///
    /// # Errors
    /// Returns an error if the conversion fails.
    fn to_primitive(&self) -> Result<P, ScalarCastError>;

    /// Consumes into a converted primitive value.
    ///
    /// # Panics
    /// Panics if the conversion fails.
    fn into_primitive_unchecked(self) -> P {
        self.into_primitive()
            .expect("conversion to primitive type failed")
    }
}

/// An interface for conversion from a primitive value.
///
/// This trait is used to obtain a [`Scalar`], which has its own representation,
/// from a primitive type ([`f32`], [`f64`], [`i32`], [`i64`]). The opposite
/// trait is [`IntoPrimitive`].
pub trait FromPrimitive<P: Debug>: Sized {
    /// Converts from a primitive value.
    ///
    /// # Errors
    /// Returns an error if the conversion fails.
    fn from_primitive(prim: P) -> Result<Self, PrimitiveCastError<P>>;

    /// Converts from a primitive value.
    ///
    /// If a direct conversion cannot be obtained from the primitive value, it is clamped.
    fn from_primitive_bounded(prim: P) -> Self;
}

impl IntoPrimitive<i32> for Scalar {
    fn into_primitive(self) -> Result<i32, ScalarCastError> {
        let r = self.0;
        r.to_integer().to_i32().ok_or(ScalarCastError {
            weight: r,
            target: PrimitiveType::I32,
        })
    }

    fn to_primitive(&self) -> Result<i32, ScalarCastError> {
        self.clone().into_primitive()
    }
}

impl FromPrimitive<i32> for Scalar {
    fn from_primitive(prim: i32) -> Result<Self, PrimitiveCastError<i32>> {
        let i = BigUint::try_from(prim).map_err(|_| PrimitiveCastError(prim))?;
        Ok(Self(Ratio::from_integer(i)))
    }

    fn from_primitive_bounded(prim: i32) -> Self {
        Self::from_primitive(prim).unwrap_or_else(|_| Self(Ratio::zero()))
    }
}

impl IntoPrimitive<i64> for Scalar {
    fn into_primitive(self) -> Result<i64, ScalarCastError> {
        let i = self.0;
        i.to_integer().to_i64().ok_or(ScalarCastError {
            weight: i,
            target: PrimitiveType::I64,
        })
    }

    fn to_primitive(&self) -> Result<i64, ScalarCastError> {
        self.clone().into_primitive()
    }
}

impl FromPrimitive<i64> for Scalar {
    fn from_primitive(prim: i64) -> Result<Self, PrimitiveCastError<i64>> {
        let i = BigUint::try_from(prim).map_err(|_| PrimitiveCastError(prim))?;
        Ok(Self(Ratio::from_integer(i)))
    }

    fn from_primitive_bounded(prim: i64) -> Self {
        Self::from_primitive(prim).unwrap_or_else(|_| Self(Ratio::zero()))
    }
}

impl IntoPrimitive<f32> for Scalar {
    fn into_primitive(self) -> Result<f32, ScalarCastError> {
        let r = self.to_ratio();
        ratio_to_float(&r).ok_or(ScalarCastError {
            weight: self.0,
            target: PrimitiveType::F32,
        })
    }

    fn to_primitive(&self) -> Result<f32, ScalarCastError> {
        self.clone().into_primitive()
    }
}

impl FromPrimitive<f32> for Scalar {
    fn from_primitive(prim: f32) -> Result<Self, PrimitiveCastError<f32>> {
        let r = Ratio::from_float(prim).ok_or(PrimitiveCastError(prim))?;
        r.try_into().map_err(|_| PrimitiveCastError(prim))
    }

    fn from_primitive_bounded(prim: f32) -> Self {
        Self::from_float_bounded(prim)
    }
}

impl IntoPrimitive<f64> for Scalar {
    fn into_primitive(self) -> Result<f64, ScalarCastError> {
        let r = self.to_ratio();
        ratio_to_float(&r).ok_or(ScalarCastError {
            weight: self.0,
            target: PrimitiveType::F64,
        })
    }

    fn to_primitive(&self) -> Result<f64, ScalarCastError> {
        self.clone().into_primitive()
    }
}

impl FromPrimitive<f64> for Scalar {
    fn from_primitive(prim: f64) -> Result<Self, PrimitiveCastError<f64>> {
        let r = Ratio::from_float(prim).ok_or(PrimitiveCastError(prim))?;
        r.try_into().map_err(|_| PrimitiveCastError(prim))
    }

    fn from_primitive_bounded(prim: f64) -> Self {
        Self::from_float_bounded(prim)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratio_conversion() {
        let (numer, denom) = (1_u8, 2_u8);
        let expected_ratio = Ratio::new(BigInt::from(numer), BigInt::from(denom));
        let actual_ratio = Scalar::new(numer, denom).into();
        assert_eq!(expected_ratio, actual_ratio);
    }

    #[test]
    fn test_ratio_conversion_ok() {
        let (numer, denom) = (1_u8, 2_u8);
        let ratio = Ratio::new(BigInt::from(numer), BigInt::from(denom));
        let sc_res = Scalar::try_from(ratio);
        assert!(sc_res.is_ok());
        assert_eq!(sc_res.unwrap(), Scalar::new(numer, denom));
    }

    #[test]
    fn test_ratio_conversion_err() {
        let neg_ratio = Ratio::new(BigInt::from(-1), BigInt::from(2));
        let sc_res = Scalar::try_from(neg_ratio);
        assert!(sc_res.is_err());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_scalar_f32() {
        let prim_sc_pairs = vec![
            (0_f32, Scalar::from_integer(0_u8)),
            (2_f32, Scalar::from_integer(2_u8)),
            (0.5_f32, Scalar::new(1_u8, 2_u8)),
        ];
        for (prim, sc) in prim_sc_pairs {
            let converted_sc = Scalar::from_primitive(prim);
            assert!(converted_sc.is_ok());
            assert_eq!(converted_sc.unwrap(), sc);

            let converted_sc = Scalar::from_primitive_bounded(prim);
            assert_eq!(converted_sc, sc);

            let converted_prim: f32 = sc.into_primitive_unchecked();
            assert_eq!(converted_prim, prim);
        }
    }

    #[test]
    fn test_scalar_f32_from_weird_prims() {
        let prim_pairs = vec![
            (f32::INFINITY, f32::MAX),
            (-1_f32, 0_f32),
            (f32::NAN, 0_f32),
        ];
        for (weird, fine) in prim_pairs {
            let weird_res = Scalar::from_primitive(weird);
            assert!(weird_res.is_err());

            let bounded = Scalar::from_primitive_bounded(weird);
            let fine_res = Scalar::try_from(Ratio::from_float(fine).unwrap());
            assert!(fine_res.is_ok());
            assert_eq!(bounded, fine_res.unwrap());
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_scalar_f64() {
        let prim_sc_pairs = vec![
            (0_f64, Scalar::from_integer(0_u8)),
            (2_f64, Scalar::from_integer(2_u8)),
            (0.5_f64, Scalar::new(1_u8, 2_u8)),
        ];
        for (prim, sc) in prim_sc_pairs {
            let converted_sc = Scalar::from_primitive(prim);
            assert!(converted_sc.is_ok());
            assert_eq!(converted_sc.unwrap(), sc);

            let converted_sc = Scalar::from_primitive_bounded(prim);
            assert_eq!(converted_sc, sc);

            let converted_prim: f64 = sc.into_primitive_unchecked();
            assert_eq!(converted_prim, prim);
        }
    }

    #[test]
    fn test_scalar_f64_from_weird_prims() {
        let prim_pairs = vec![
            (f64::INFINITY, f64::MAX),
            (-1_f64, 0_f64),
            (f64::NAN, 0_f64),
        ];
        for (weird, fine) in prim_pairs {
            let weird_res = Scalar::from_primitive(weird);
            assert!(weird_res.is_err());

            let bounded = Scalar::from_primitive_bounded(weird);
            let fine_res = Scalar::try_from(Ratio::from_float(fine).unwrap());
            assert!(fine_res.is_ok());
            assert_eq!(bounded, fine_res.unwrap());
        }
    }

    #[test]
    fn test_scalar_i32() {
        let prim_sc_pairs = vec![
            (0_i32, Scalar::from_integer(0_u8)),
            (2_i32, Scalar::from_integer(2_u8)),
        ];
        for (prim, sc) in prim_sc_pairs {
            let converted_sc = Scalar::from_primitive(prim);
            assert!(converted_sc.is_ok());
            assert_eq!(converted_sc.unwrap(), sc);

            let converted_sc = Scalar::from_primitive_bounded(prim);
            assert_eq!(converted_sc, sc);

            let converted_prim: i32 = sc.into_primitive_unchecked();
            assert_eq!(converted_prim, prim);
        }
    }

    #[test]
    fn test_scalar_i64() {
        let prim_sc_pairs = vec![
            (0_i64, Scalar::from_integer(0_u8)),
            (2_i64, Scalar::from_integer(2_u8)),
        ];
        for (prim, sc) in prim_sc_pairs {
            let converted_sc = Scalar::from_primitive(prim);
            assert!(converted_sc.is_ok());
            assert_eq!(converted_sc.unwrap(), sc);

            let converted_sc = Scalar::from_primitive_bounded(prim);
            assert_eq!(converted_sc, sc);

            let converted_prim: i64 = sc.into_primitive_unchecked();
            assert_eq!(converted_prim, prim);
        }
    }
}
