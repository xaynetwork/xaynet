//! Loading and validation of settings.
//!
//! Values defined in the configuration file can be overridden by environment variables. Examples of
//! configuration files can be found in the `configs/` directory located in the repository root.

use std::{fmt, path::PathBuf};

use config::{Config, ConfigError, Environment};
use serde::de::{self, Deserializer, Visitor};
use thiserror::Error;
use tracing_subscriber::filter::EnvFilter;
use validator::{Validate, ValidationError, ValidationErrors};

use crate::mask::config::{BoundType, DataType, GroupType, MaskConfig, ModelType};

#[derive(Error, Debug)]
/// An error related to loading and validation of settings.
pub enum SettingsError {
    #[error("configuration loading failed: {0}")]
    Loading(#[from] ConfigError),
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationErrors),
}

#[derive(Debug, Validate, Deserialize)]
/// The combined settings.
///
/// Each section in the configuration file corresponds to the identically named settings field.
pub struct Settings {
    #[validate]
    pub api: ApiSettings,
    #[validate]
    pub pet: PetSettings,
    pub mask: MaskSettings,
    pub log: LoggingSettings,
}

impl Settings {
    /// Loads and validates the settings via a configuration file.
    ///
    /// # Errors
    /// Fails when the loading of the configuration file or its validation failed.
    pub fn new(path: PathBuf) -> Result<Self, SettingsError> {
        let settings: Settings = Self::load(path)?;
        settings.validate()?;
        Ok(settings)
    }

    fn load(path: PathBuf) -> Result<Self, ConfigError> {
        let mut config = Config::new();
        config.merge(config::File::from(path))?;
        config.merge(Environment::with_prefix("xaynet").separator("__"))?;
        config.try_into()
    }
}

#[derive(Debug, Validate, Deserialize, Clone, Copy)]
#[validate(schema(function = "validate_fractions"))]
/// PET protocol settings.
pub struct PetSettings {
    #[validate(range(min = 1))]
    /// The minimal number of participants selected for computing the unmasking sum. The value must
    /// be greater or equal to `1` (i.e. `min_sum_count >= 1`), otherwise the PET protocol will be
    /// broken.
    ///
    /// This parameter should only be used to enforce security constraints. To control the expected
    /// number of sum participants, the `sum` fraction should be adjusted wrt the total number of
    /// `expected_participants`.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [pet]
    /// min_sum_count = 1
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAYNET_PET__MIN_SUM_COUNT=1
    /// ```
    pub min_sum_count: usize,

    #[validate(range(min = 3))]
    /// The expected fraction of participants selected for submitting an updated local model for
    /// aggregation. The value must be greater or equal to `3` (i.e. `min_update_count >= 3`),
    /// otherwise the PET protocol will be broken.
    ///
    /// This parameter should only be used to enforce security constraints. To control the expected
    /// number of update participants, the `update` fraction should be adjusted wrt the total number
    /// of `expected_participants`.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [pet]
    /// min_update_count = 3
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAYNET_PET__MIN_UPDATE_COUNT=3
    /// ```
    pub min_update_count: usize,

    /// The minimum amount of time reserved for processing messages in the `sum`
    /// and `sum2` phases, in seconds.
    ///
    /// Defaults to 0 i.e. `sum` and `sum2` phases end *as soon as*
    /// [`min_sum_count`] messages have been processed. Set this higher to allow
    /// for the possibility of more than [`min_sum_count`] messages to be
    /// processed in the `sum` and `sum2` phases.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [pet]
    /// min_sum_time = 5
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAYNET_PET__MIN_SUM_TIME=5
    /// ```
    pub min_sum_time: u64,

    /// The minimum amount of time reserved for processing messages in the
    /// `update` phase, in seconds.
    ///
    /// Defaults to 0 i.e. `update` phase ends *as soon as* [`min_update_count`]
    /// messages have been processed. Set this higher to allow for the
    /// possibility of more than [`min_update_count`] messages to be processed
    /// in the `update` phase.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [pet]
    /// min_update_time = 10
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAYNET_PET__MIN_UPDATE_TIME=10
    /// ```
    pub min_update_time: u64,

    /// The expected fraction of participants selected for computing the unmasking sum. The value
    /// must be between `0` and `1` (i.e. `0 < sum < 1`).
    ///
    /// Additionally, it is enforced that `0 < sum + update - sum*update < 1` to avoid pathological
    /// cases of deadlocks.
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
    /// XAYNET_PET__SUM=0.01
    /// ```
    pub sum: f64,

    /// The expected fraction of participants selected for submitting an updated local model for
    /// aggregation. The value must be between `0` and `1` (i.e. `0 < update < 1`).
    ///
    /// Additionally, it is enforced that `0 < sum + update - sum*update < 1` to avoid pathological
    /// cases of deadlocks.
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
    /// XAYNET_PET__UPDATE=0.01
    /// ```
    pub update: f64,

    #[validate(range(min = 1))]
    /// The total number of participants that are expected by the coordinator. The value must be a
    /// positive integer (i.e. `expected_participants >= 1`).
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [pet]
    /// expected_participants = 10
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAYNET_PET__EXPECTED_PARTICIPANTS=10
    /// ```
    pub expected_participants: usize,
}

impl Default for PetSettings {
    fn default() -> Self {
        Self {
            min_sum_count: 1_usize,
            min_update_count: 3_usize,
            min_sum_time: 0_u64,
            min_update_time: 0_u64,
            sum: 0.01_f64,
            update: 0.1_f64,
            expected_participants: 10,
        }
    }
}

/// Checks pathological cases of deadlocks.
fn validate_fractions(s: &PetSettings) -> Result<(), ValidationError> {
    if 0. < s.sum
        && s.sum < 1.
        && 0. < s.update
        && s.update < 1.
        && 0. < s.sum + s.update - s.sum * s.update
        && s.sum + s.update - s.sum * s.update < 1.
    {
        Ok(())
    } else {
        Err(ValidationError::new("starvation"))
    }
}

#[derive(Debug, Validate, Deserialize, Clone, Copy)]
/// REST API settings.
pub struct ApiSettings {
    /// The address to which the REST API should be bound.
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [api]
    /// bind_address = "0.0.0.0:8081"
    /// # or
    /// bind_address = "127.0.0.1:8081"
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAYNET_API__BIND_ADDRESS=127.0.0.1:8081
    /// ```
    pub bind_address: std::net::SocketAddr,
}

#[derive(Debug, Validate, Deserialize, Clone, Copy)]
/// Masking settings.
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
    /// XAYNET_MASK__GROUP_TYPE=Integer
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
    /// XAYNET_MASK__DATA_TYPE=F32
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
    /// XAYNET_MASK__BOUND_TYPE=B0
    /// ```
    pub bound_type: BoundType,

    /// The maximum number of models to be aggregated.
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
    /// XAYNET_MASK__MODEL_TYPE=M3
    /// ```
    pub model_type: ModelType,
}

impl Default for MaskSettings {
    fn default() -> Self {
        Self {
            group_type: GroupType::Prime,
            data_type: DataType::F32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        }
    }
}

impl From<MaskSettings> for MaskConfig {
    fn from(
        MaskSettings {
            group_type,
            data_type,
            bound_type,
            model_type,
        }: MaskSettings,
    ) -> MaskConfig {
        MaskConfig {
            group_type,
            data_type,
            bound_type,
            model_type,
        }
    }
}

#[derive(Debug, Deserialize)]
/// Logging settings.
pub struct LoggingSettings {
    /// A comma-separated list of logging directives. More information about logging directives
    /// can be found [here].
    ///
    /// # Examples
    ///
    /// **TOML**
    /// ```text
    /// [log]
    /// filter = "info"
    /// ```
    ///
    /// **Environment variable**
    /// ```text
    /// XAYNET_LOG__FILTER=info
    /// ```
    ///
    /// [here]: https://docs.rs/tracing-subscriber/0.2.6/tracing_subscriber/filter/struct.EnvFilter.html#directives
    #[serde(deserialize_with = "deserialize_env_filter")]
    pub filter: EnvFilter,
}

fn deserialize_env_filter<'de, D>(deserializer: D) -> Result<EnvFilter, D::Error>
where
    D: Deserializer<'de>,
{
    struct EnvFilterVisitor;

    impl<'de> Visitor<'de> for EnvFilterVisitor {
        type Value = EnvFilter;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "a valid tracing filter directive: https://docs.rs/tracing-subscriber/0.2.6/tracing_subscriber/filter/struct.EnvFilter.html#directives")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            EnvFilter::try_new(value)
                .map_err(|_| de::Error::invalid_value(serde::de::Unexpected::Str(value), &self))
        }
    }

    deserializer.deserialize_str(EnvFilterVisitor)
}
