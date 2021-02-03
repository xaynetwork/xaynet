//! Scalar representation and conversion.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]: ../index.html

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
use std::{convert::TryFrom, fmt::Debug};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Hash, From, Into, Serialize, Deserialize)]
/// A numerical representation of a machine learning scalar.
pub struct Scalar(Ratio<BigUint>);

impl Scalar {
    /// Constructs a new `Scalar` from the given numerator and denominator.
    pub fn new<U>(numer: U, denom: U) -> Self
    where
        U: Unsigned + Into<BigUint>,
    {
        Self(Ratio::new(numer.into(), denom.into()))
    }

    /// Constructs a `Scalar` of unit value.
    pub fn unit() -> Self {
        Self(Ratio::one())
    }

    /// Convenience method for conversion into a non-negative ratio of `BigInt`.
    pub(crate) fn into_ratio(self) -> Ratio<BigInt> {
        let (numer, denom) = self.0.into();
        Ratio::new(numer.into(), denom.into())
    }

    /// Convenience method for conversion to a non-negative ratio of `BigInt`.
    pub(crate) fn to_ratio(&self) -> Ratio<BigInt> {
        let numer = self.0.numer().clone();
        let denom = self.0.denom().clone();
        Ratio::new(numer.into(), denom.into())
    }

    fn from_ratio(r: Ratio<BigInt>) -> anyhow::Result<Self> {
        let (inumer, idenom) = r.into();
        let unumer = BigUint::try_from(inumer)?;
        let udenom = BigUint::try_from(idenom)?;
        Ok(Self(Ratio::new(unumer, udenom)))
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
        let r = self.0.clone();
        r.to_integer().to_i32().ok_or(ScalarCastError {
            weight: r,
            target: PrimitiveType::I32,
        })
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
        let i = self.0.clone();
        i.to_integer().to_i64().ok_or(ScalarCastError {
            weight: i,
            target: PrimitiveType::I64,
        })
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
        ratio_to_float::<f32>(&r).ok_or(ScalarCastError {
            weight: self.0,
            target: PrimitiveType::F32,
        })
    }

    fn to_primitive(&self) -> Result<f32, ScalarCastError> {
        let r = self.to_ratio();
        ratio_to_float::<f32>(&r).ok_or(ScalarCastError {
            weight: self.0.clone(),
            target: PrimitiveType::F32,
        })
    }
}

impl FromPrimitive<f32> for Scalar {
    fn from_primitive(prim: f32) -> Result<Self, PrimitiveCastError<f32>> {
        let r = Ratio::from_float(prim).ok_or(PrimitiveCastError(prim))?;
        Ok(Self::from_ratio(r).map_err(|_| PrimitiveCastError(prim))?)
    }

    fn from_primitive_bounded(prim: f32) -> Self {
        let r = float_to_ratio_bounded::<f32>(prim);
        // safe unwrap: bounded non-negative ratio r
        Self::from_ratio(r).unwrap()
    }
}

impl IntoPrimitive<f64> for Scalar {
    fn into_primitive(self) -> Result<f64, ScalarCastError> {
        let r = self.to_ratio();
        ratio_to_float::<f64>(&r).ok_or(ScalarCastError {
            weight: self.0,
            target: PrimitiveType::F64,
        })
    }

    fn to_primitive(&self) -> Result<f64, ScalarCastError> {
        let r = self.to_ratio();
        ratio_to_float::<f64>(&r).ok_or(ScalarCastError {
            weight: self.0.clone(),
            target: PrimitiveType::F64,
        })
    }
}

impl FromPrimitive<f64> for Scalar {
    fn from_primitive(prim: f64) -> Result<Self, PrimitiveCastError<f64>> {
        let r = Ratio::from_float(prim).ok_or(PrimitiveCastError(prim))?;
        Ok(Self::from_ratio(r).map_err(|_| PrimitiveCastError(prim))?)
    }

    fn from_primitive_bounded(prim: f64) -> Self {
        let r = float_to_ratio_bounded::<f64>(prim);
        // safe unwrap: bounded non-negative ratio r
        Self::from_ratio(r).unwrap()
    }
}

/// Converts the primitive floating point value into a numerical value.
///
/// Maps positive/negative infinity to max/min of the primitive data type and NaN to zero.
pub(crate) fn float_to_ratio_bounded<F: FloatCore>(f: F) -> Ratio<BigInt> {
    if f.is_nan() {
        Ratio::<BigInt>::zero()
    } else {
        let finite_f = clamp(f, F::zero(), F::max_value());
        // safe unwrap: clamped weight is guaranteed to be finite
        Ratio::<BigInt>::from_float(finite_f).unwrap()
    }
}
