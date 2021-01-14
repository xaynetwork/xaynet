//! Masked objects.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]: crate::mask

pub mod serialization;

use std::iter::Iterator;

use num::bigint::BigUint;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mask::config::{MaskConfig, MaskConfigPair};

#[derive(Error, Debug)]
#[error("the mask object is invalid: data is incompatible with the masking configuration")]
/// Errors related to invalid mask objects.
pub struct InvalidMaskObjectError;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
/// A *mask vector* which represents a masked model or its corresponding mask.
pub struct MaskVect {
    pub data: Vec<BigUint>,
    pub config: MaskConfig,
}

impl MaskVect {
    /// Creates a new mask vector from the given data and masking configuration.
    pub fn new_unchecked(config: MaskConfig, data: Vec<BigUint>) -> Self {
        Self { data, config }
    }

    /// Creates a new mask vector from the given data and masking configuration.
    ///
    /// # Errors
    /// Fails if the elements of the mask object don't conform to the given masking configuration.
    pub fn new(config: MaskConfig, data: Vec<BigUint>) -> Result<Self, InvalidMaskObjectError> {
        let obj = Self::new_unchecked(config, data);
        if obj.is_valid() {
            Ok(obj)
        } else {
            Err(InvalidMaskObjectError)
        }
    }

    /// Creates a new empty mask vector of given size and masking configuration.
    pub fn empty(config: MaskConfig, size: usize) -> Self {
        Self {
            data: Vec::with_capacity(size),
            config,
        }
    }

    /// Checks if the elements of this mask vector conform to the masking configuration.
    pub fn is_valid(&self) -> bool {
        let order = self.config.order();
        self.data.iter().all(|i| i < &order)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
/// A *mask unit* which represents a masked scalar or its corresponding mask.
pub struct MaskUnit {
    pub data: BigUint,
    pub config: MaskConfig,
}

impl From<&MaskUnit> for MaskVect {
    fn from(mask_unit: &MaskUnit) -> Self {
        Self::new_unchecked(mask_unit.config, vec![mask_unit.data.clone()])
    }
}

impl From<MaskUnit> for MaskVect {
    fn from(mask_unit: MaskUnit) -> Self {
        Self::new_unchecked(mask_unit.config, vec![mask_unit.data])
    }
}

impl MaskUnit {
    /// Creates a new mask unit from the given mask and masking configuration.
    pub fn new_unchecked(config: MaskConfig, data: BigUint) -> Self {
        Self { data, config }
    }

    /// Creates a new mask unit from the given mask and masking configuration.
    ///
    /// # Errors
    /// Fails if the mask unit doesn't conform to the given masking configuration.
    pub fn new(config: MaskConfig, data: BigUint) -> Result<Self, InvalidMaskObjectError> {
        let obj = Self::new_unchecked(config, data);
        if obj.is_valid() {
            Ok(obj)
        } else {
            Err(InvalidMaskObjectError)
        }
    }

    /// Creates a new mask unit of given masking configuration with default value `1`.
    pub fn default(config: MaskConfig) -> Self {
        Self {
            data: BigUint::from(1_u8),
            config,
        }
    }

    /// Checks if the data value conforms to the masking configuration.
    pub fn is_valid(&self) -> bool {
        self.data < self.config.order()
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
/// A mask object consisting of a vector part and unit part.
pub struct MaskObject {
    pub vect: MaskVect,
    pub unit: MaskUnit,
}

impl MaskObject {
    /// Creates a new mask object from the given vector and unit.
    pub fn new_unchecked(vect: MaskVect, unit: MaskUnit) -> Self {
        Self { vect, unit }
    }

    /// Creates a new mask object from the given vector, unit and masking configurations.
    pub fn new(
        config: MaskConfigPair,
        data_vect: Vec<BigUint>,
        data_unit: BigUint,
    ) -> Result<Self, InvalidMaskObjectError> {
        let vect = MaskVect::new(config.vect, data_vect)?;
        let unit = MaskUnit::new(config.unit, data_unit)?;
        Ok(Self { vect, unit })
    }

    /// Creates a new empty mask object of given size and masking configurations.
    pub fn empty(config: MaskConfigPair, size: usize) -> Self {
        Self {
            vect: MaskVect::empty(config.vect, size),
            unit: MaskUnit::default(config.unit),
        }
    }

    /// Checks if this mask object conforms to the masking configurations.
    pub fn is_valid(&self) -> bool {
        self.vect.is_valid() && self.unit.is_valid()
    }
}
