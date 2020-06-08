use std::{
    fmt::Debug,
    iter::{FromIterator, IntoIterator},
    slice::{Iter, IterMut},
};

use derive_more::{Display, From, Index, IndexMut, Into};
use num::{
    bigint::BigInt,
    clamp,
    rational::Ratio,
    traits::{float::FloatCore, identities::Zero, ToPrimitive},
};
use thiserror::Error;

/// Represent a model.
#[derive(Debug, Clone, PartialEq, Hash, From, Index, IndexMut, Into, Serialize, Deserialize)]
pub struct Model(Vec<Ratio<BigInt>>);

#[allow(clippy::len_without_is_empty)]
impl Model {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> Iter<Ratio<BigInt>> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<Ratio<BigInt>> {
        self.0.iter_mut()
    }
}

impl FromIterator<Ratio<BigInt>> for Model {
    fn from_iter<I: IntoIterator<Item = Ratio<BigInt>>>(iter: I) -> Self {
        let data: Vec<Ratio<BigInt>> = iter.into_iter().collect();
        Model(data)
    }
}

impl IntoIterator for Model {
    type Item = Ratio<BigInt>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Display)]
enum PrimitiveType {
    F32,
    F64,
    I32,
    I64,
}

#[derive(Error, Debug)]
#[error("Could not convert weight {weight} to primitive type {target}")]
pub struct ModelCastError {
    weight: Ratio<BigInt>,
    target: PrimitiveType,
}

#[derive(Error, Debug)]
#[error("Could not convert primitive type {0:?} to model weight")]
pub struct PrimitiveCastError<P: Debug>(P);

/// Convert this type into a an iterator of type `P`. This trait is
/// used to convert a [`Model`], which has its own internal
/// representation of the weights into primitive types (`f64`, `f32`,
/// `i32` `i64`).
pub trait IntoPrimitives<P: 'static>: Sized {
    /// Consume this model and into an iterator that yields `Ok(P)`
    /// for each model weight that can be converted to `P`, and
    /// `Err(ModelCastError)` for each weight that cannot be converted
    /// to `P`.
    fn into_primitives(self) -> Box<dyn Iterator<Item = Result<P, ModelCastError>>>;

    /// Consume this model and into an iterator that yields `P` values.
    ///
    /// # Panics
    ///
    /// This method panics if a model weight cannot be converted into
    /// `P`.
    fn into_primitives_unchecked(self) -> Box<dyn Iterator<Item = P>> {
        Box::new(
            self.into_primitives()
                .map(|res| res.expect("conversion to primitive type failed")),
        )
    }
}

/// Convert a stream of numerical primitive types (`i32`, `i64`,
/// `f32`, `f64`) into this type.
pub trait FromPrimitives<P: Debug>: Sized {
    /// Consume an iterator that yields `P`, into a model. If a `P`
    /// cannot be converted to a model weight, this method fails.
    fn from_primitives<I: Iterator<Item = P>>(iter: I) -> Result<Self, PrimitiveCastError<P>>;

    /// Consume an iterator that yields `P` values into a model. If a
    /// `P` cannot be directly converted into a model weight because it is not finite, it is clamped.
    fn from_primitives_bounded<I: Iterator<Item = P>>(iter: I) -> Self;
}

impl IntoPrimitives<i32> for Model {
    fn into_primitives(self) -> Box<dyn Iterator<Item = Result<i32, ModelCastError>>> {
        Box::new(self.0.into_iter().map(|i| {
            i.to_integer().to_i32().ok_or_else(|| ModelCastError {
                weight: i,
                target: PrimitiveType::I32,
            })
        }))
    }
}

impl FromPrimitives<i32> for Model {
    fn from_primitives<I: Iterator<Item = i32>>(iter: I) -> Result<Self, PrimitiveCastError<i32>> {
        Ok(iter.map(|p| Ratio::from_integer(BigInt::from(p))).collect())
    }

    fn from_primitives_bounded<I: Iterator<Item = i32>>(iter: I) -> Self {
        Self::from_primitives(iter).unwrap()
    }
}

impl IntoPrimitives<i64> for Model {
    fn into_primitives(self) -> Box<dyn Iterator<Item = Result<i64, ModelCastError>>> {
        Box::new(self.0.into_iter().map(|i| {
            i.to_integer().to_i64().ok_or_else(|| ModelCastError {
                weight: i,
                target: PrimitiveType::I64,
            })
        }))
    }
}

impl FromPrimitives<i64> for Model {
    fn from_primitives<I: Iterator<Item = i64>>(iter: I) -> Result<Self, PrimitiveCastError<i64>> {
        Ok(iter.map(|p| Ratio::from_integer(BigInt::from(p))).collect())
    }

    fn from_primitives_bounded<I: Iterator<Item = i64>>(iter: I) -> Self {
        Self::from_primitives(iter).unwrap()
    }
}

impl IntoPrimitives<f32> for Model {
    fn into_primitives(self) -> Box<dyn Iterator<Item = Result<f32, ModelCastError>>> {
        let iter = self.0.into_iter().map(|r| {
            ratio_to_float::<f32>(&r).ok_or_else(|| ModelCastError {
                weight: r,
                target: PrimitiveType::F32,
            })
        });
        Box::new(iter)
    }
}

impl FromPrimitives<f32> for Model {
    fn from_primitives<I: Iterator<Item = f32>>(iter: I) -> Result<Self, PrimitiveCastError<f32>> {
        iter.map(|f| Ratio::from_float(f).ok_or_else(|| PrimitiveCastError(f)))
            .collect()
    }

    fn from_primitives_bounded<I: Iterator<Item = f32>>(iter: I) -> Self {
        iter.map(float_to_ratio_bounded::<f32>).collect()
    }
}

impl IntoPrimitives<f64> for Model {
    fn into_primitives(self) -> Box<dyn Iterator<Item = Result<f64, ModelCastError>>> {
        let iter = self.0.into_iter().map(|r| {
            ratio_to_float::<f64>(&r).ok_or_else(|| ModelCastError {
                weight: r,
                target: PrimitiveType::F64,
            })
        });
        Box::new(iter)
    }
}

impl FromPrimitives<f64> for Model {
    fn from_primitives<I: Iterator<Item = f64>>(iter: I) -> Result<Self, PrimitiveCastError<f64>> {
        iter.map(|f| Ratio::from_float(f).ok_or_else(|| PrimitiveCastError(f)))
            .collect()
    }

    fn from_primitives_bounded<I: Iterator<Item = f64>>(iter: I) -> Self {
        iter.map(float_to_ratio_bounded::<f64>).collect()
    }
}

fn ratio_to_float<F: FloatCore>(ratio: &Ratio<BigInt>) -> Option<F> {
    let min_value = Ratio::from_float(F::min_value()).unwrap();
    let max_value = Ratio::from_float(F::max_value()).unwrap();
    if ratio < &min_value || ratio > &max_value {
        return None;
    }

    let mut numer = ratio.numer().clone();
    let mut denom = ratio.denom().clone();
    // safe loop: terminates after at most bit-length of ratio iterations
    loop {
        if let (Some(n), Some(d)) = (F::from(numer.clone()), F::from(denom.clone())) {
            if n == F::zero() || d == F::zero() {
                break Some(F::zero());
            } else {
                let float = n / d;
                if float.is_finite() {
                    break Some(float);
                }
            }
        } else {
            numer >>= 1_usize;
            denom >>= 1_usize;
        }
    }
}

/// Cast the given float to a ratio. Positive/negative infinity is
/// mapped to max/min and NaN to zero.
fn float_to_ratio_bounded<F: FloatCore>(f: F) -> Ratio<BigInt> {
    if f.is_nan() {
        Ratio::<BigInt>::zero()
    } else {
        let finite_f = clamp(f, F::min_value(), F::max_value());
        // safe unwrap: clamped weight is guaranteed to be finite
        Ratio::<BigInt>::from_float(finite_f).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter;

    type R = Ratio<BigInt>;

    #[test]
    fn test_model_f32() {
        let expected_primitives = vec![-1_f32, 0_f32, 1_f32];
        let expected_model = Model::from(vec![
            R::from_float(-1_f32).unwrap(),
            R::zero(),
            R::from_float(1_f32).unwrap(),
        ]);

        let actual_model = Model::from_primitives(expected_primitives.iter().cloned()).unwrap();
        assert_eq!(actual_model, expected_model);

        let actual_model = Model::from_primitives_bounded(expected_primitives.iter().cloned());
        assert_eq!(actual_model, expected_model);

        let actual_primitives: Vec<f32> = expected_model.into_primitives_unchecked().collect();
        assert_eq!(actual_primitives, expected_primitives);
    }

    #[test]
    fn test_model_f64() {
        let expected_primitives = vec![-1_f64, 0_f64, 1_f64];
        let expected_model = Model::from(vec![
            R::from_float(-1_f64).unwrap(),
            R::zero(),
            R::from_float(1_f64).unwrap(),
        ]);

        let actual_model = Model::from_primitives(expected_primitives.iter().cloned()).unwrap();
        assert_eq!(actual_model, expected_model);

        let actual_model = Model::from_primitives_bounded(expected_primitives.iter().cloned());
        assert_eq!(actual_model, expected_model);

        let actual_primitives: Vec<f64> = expected_model.into_primitives_unchecked().collect();
        assert_eq!(actual_primitives, expected_primitives);
    }

    #[test]
    fn test_model_f32_from_weird_primitives() {
        // +infinity
        assert!(Model::from_primitives(iter::once(f32::INFINITY)).is_err());
        assert_eq!(
            Model::from_primitives_bounded(iter::once(f32::INFINITY)),
            vec![R::from_float(f32::MAX).unwrap()].into()
        );

        // -infinity
        assert!(Model::from_primitives(iter::once(f32::NEG_INFINITY)).is_err());
        assert_eq!(
            Model::from_primitives_bounded(iter::once(f32::NEG_INFINITY)),
            vec![R::from_float(f32::MIN).unwrap()].into()
        );

        // NaN
        assert!(Model::from_primitives(iter::once(f32::NAN)).is_err());
        assert_eq!(
            Model::from_primitives_bounded(iter::once(f32::NAN)),
            vec![R::zero()].into()
        );
    }

    #[test]
    fn test_model_f64_from_weird_primitives() {
        // +infinity
        assert!(Model::from_primitives(iter::once(f64::INFINITY)).is_err());
        assert_eq!(
            Model::from_primitives_bounded(iter::once(f64::INFINITY)),
            vec![R::from_float(f64::MAX).unwrap()].into()
        );

        // -infinity
        assert!(Model::from_primitives(iter::once(f64::NEG_INFINITY)).is_err());
        assert_eq!(
            Model::from_primitives_bounded(iter::once(f64::NEG_INFINITY)),
            vec![R::from_float(f64::MIN).unwrap()].into()
        );

        // NaN
        assert!(Model::from_primitives(iter::once(f64::NAN)).is_err());
        assert_eq!(
            Model::from_primitives_bounded(iter::once(f64::NAN)),
            vec![R::zero()].into()
        );
    }

    #[test]
    fn test_model_i32() {
        let expected_primitives = vec![-1_i32, 0_i32, 1_i32];
        let expected_model = Model::from(vec![
            R::from_integer(BigInt::from(-1_i32)),
            R::zero(),
            R::from_integer(BigInt::from(1_i32)),
        ]);

        let actual_model = Model::from_primitives(expected_primitives.iter().cloned()).unwrap();
        assert_eq!(actual_model, expected_model);

        let actual_model = Model::from_primitives_bounded(expected_primitives.iter().cloned());
        assert_eq!(actual_model, expected_model);

        let actual_primitives: Vec<i32> = expected_model.into_primitives_unchecked().collect();
        assert_eq!(actual_primitives, expected_primitives);
    }

    #[test]
    fn test_model_i64() {
        let expected_primitives = vec![-1_i64, 0_i64, 1_i64];
        let expected_model = Model::from(vec![
            R::from_integer(BigInt::from(-1_i64)),
            R::zero(),
            R::from_integer(BigInt::from(1_i64)),
        ]);

        let actual_model = Model::from_primitives(expected_primitives.iter().cloned()).unwrap();
        assert_eq!(actual_model, expected_model);

        let actual_model = Model::from_primitives_bounded(expected_primitives.iter().cloned());
        assert_eq!(actual_model, expected_model);

        let actual_primitives: Vec<i64> = expected_model.into_primitives_unchecked().collect();
        assert_eq!(actual_primitives, expected_primitives);
    }

    #[test]
    fn test_ratio_to_float() {
        let ratio = R::from_float(0_f32).unwrap();
        assert_eq!(ratio_to_float::<f32>(&ratio).unwrap(), 0.0);
        let ratio = R::from_float(0_f64).unwrap();
        assert_eq!(ratio_to_float::<f64>(&ratio).unwrap(), 0.0);

        let ratio = R::from_float(0.1_f32).unwrap();
        assert_eq!(ratio_to_float::<f32>(&ratio).unwrap(), 0.1_f32);
        let ratio = R::from_float(0.1_f64).unwrap();
        assert_eq!(ratio_to_float::<f64>(&ratio).unwrap(), 0.1_f64);

        let f32_max = R::from_float(f32::max_value()).unwrap();
        let ratio = &f32_max * BigInt::from(10_usize) / (f32_max * BigInt::from(100_usize));
        assert_eq!(ratio_to_float::<f32>(&ratio).unwrap(), 0.1_f32);

        let f64_max = R::from_float(f64::max_value()).unwrap();
        let ratio = &f64_max * BigInt::from(10_usize) / (f64_max * BigInt::from(100_usize));
        assert_eq!(ratio_to_float::<f64>(&ratio).unwrap(), 0.1_f64);
    }
}
