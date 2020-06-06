pub(crate) mod serialization;

use std::iter::Iterator;

use num::bigint::BigUint;
use thiserror::Error;

use crate::mask::MaskConfig;

#[derive(Error, Debug)]
#[error("the mask object is invalid: data is incompatible with the masking configuration")]
pub struct InvalidMaskObject;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct MaskObject {
    pub(crate) data: Vec<BigUint>,
    pub(crate) config: MaskConfig,
}

impl MaskObject {
    pub fn new(config: MaskConfig, data: Vec<BigUint>) -> Self {
        Self { data, config }
    }

    pub fn new_checked(config: MaskConfig, data: Vec<BigUint>) -> Result<Self, InvalidMaskObject> {
        let obj = Self::new(config, data);
        if obj.is_valid() {
            Ok(obj)
        } else {
            Err(InvalidMaskObject)
        }
    }

    pub fn is_valid(&self) -> bool {
        let order = self.config.order();
        self.data.iter().all(|i| i < &order)
    }
}
