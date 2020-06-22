//! Module for loading and validating coordinator settings.
//!
//! Settings defined in the configuration file can be overridden by environment variables.
use crate::mask::{BoundType, DataType, GroupType, ModelType};
use config::{Config, ConfigError, Environment};
use thiserror::Error;
use validator::{Validate, ValidationErrors};

#[derive(Error, Debug)]
pub enum SettingsError {
    #[error("configuration loading failed: {0}")]
    Loading(#[from] ConfigError),
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
}

#[derive(Debug, Validate, Deserialize)]
#[doc(hidden)]
pub struct Settings {
    #[validate]
    pub api: ApiSettings,
    #[validate]
    pub pet: PetSettings,
    pub mask: MaskSettings,
}

#[derive(Debug, Validate, Deserialize)]
/// PET protocol settings
pub struct PetSettings {
    #[validate(range(min = 1))]
    pub min_sum: usize,
    #[validate(range(min = 3))]
    pub min_update: usize,

    #[validate(range(min = 0, max = 1.0))]
    /// The expected fraction of participants selected for computing the unmasking sum.
    /// The value must be between `0` and `1`.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [pet]
    /// sum = 0.01
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAIN_PET__SUM=0.01
    /// ```
    pub sum: f64,

    #[validate(range(min = 0, max = 1.0))]
    /// The expected fraction of participants selected for submitting an updated local model for
    /// aggregation. The value must be between `0` and `1`.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [pet]
    /// update = 0.01
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAIN__PET__UPDATE=0.01
    /// ```
    pub update: f64,
}

#[derive(Debug, Validate, Deserialize)]
/// REST API settings
pub struct ApiSettings {
    #[validate(url)]
    /// The address to which the REST API should be bound.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [api]
    /// bind_address = "localhost:8081"
    /// # or
    /// bind_address = "http://127.0.0.1:8081"
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAIN_API__BIND_ADDRESS=localhost:8081
    /// ```
    pub bind_address: String,
}

#[derive(Debug, Validate, Deserialize)]
/// Mask settings
pub struct MaskSettings {
    /// The order of the finite group.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [mask]
    /// group_type = "Integer"
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAIN_MASK__GROUND_TYPE=Integer
    /// ```
    pub group_type: GroupType,

    /// The data type of the numbers to be masked.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [mask]
    /// data_type = "F32"
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAIN_MASK__DATA_TYPE=F32
    /// ```
    pub data_type: DataType,

    /// The bounds of the numbers to be masked.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [mask]
    /// bound_type = "B0"
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAIN_MASK__BOUND_TYPE=B0
    /// ```
    pub bound_type: BoundType,

    /// The bounds of the numbers to be masked.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [mask]
    /// model_type = "M3"
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAIN_MASK__MODEL_TYPE=M3
    /// ```
    pub model_type: ModelType,
}

impl Settings {
    /// Loads and validates the coordinator settings via a configuration file.
    /// Fails when the loading of the configuration file or its validation failed.
    pub fn new(path: &str) -> Result<Self, SettingsError> {
        let settings: Settings = Self::load(path)?;
        settings.validate()?;
        Ok(settings)
    }

    /// Loads the coordinator settings via a configuration file without validation.
    /// Fails when the loading of configuration file failed.
    pub fn new_dirty(path: &str) -> Result<Self, ConfigError> {
        Self::load(path)
    }

    fn load(path: &str) -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(config::File::with_name(path))?;
        s.merge(Environment::with_prefix("xain").separator("__"))?;
        s.try_into()
    }
}
