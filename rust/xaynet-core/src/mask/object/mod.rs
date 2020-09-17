//! Masked objects.
//!
//! See the [mask module] documentation since this is a private module anyways.
//!
//! [mask module]: ../index.html

pub mod serialization;

use std::iter::Iterator;

use num::bigint::BigUint;
use thiserror::Error;

use crate::mask::config::MaskConfig;

#[derive(Error, Debug)]
#[error("the mask object is invalid: data is incompatible with the masking configuration")]
/// Errors related to invalid mask objects.
pub struct InvalidMaskObjectError;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
/// A mask object which represents either a mask or a masked model.
pub struct MaskObject {
    pub data: Vec<BigUint>,
    pub config: MaskConfig,
}

impl MaskObject {
    /// Creates a new mask object from the given masking configuration and the elements of the mask
    /// or masked model.
    pub fn new(config: MaskConfig, data: Vec<BigUint>) -> Self {
        Self { data, config }
    }

    /// Creates a new mask object from the given masking configuration and the elements of the mask
    /// or masked model.
    ///
    /// # Errors
    /// Fails if the elements of the mask object don't conform to the given masking configuration.
    pub fn new_checked(
        config: MaskConfig,
        data: Vec<BigUint>,
    ) -> Result<Self, InvalidMaskObjectError> {
        let obj = Self::new(config, data);
        if obj.is_valid() {
            Ok(obj)
        } else {
            Err(InvalidMaskObjectError)
        }
    }

    /// Checks if the elements of this mask object conform to the given masking configuration.
    pub fn is_valid(&self) -> bool {
        let order = self.config.order();
        self.data.iter().all(|i| i < &order)
    }
}
