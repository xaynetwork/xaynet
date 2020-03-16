use crate::common::settings::LoggingSettings;
use config::{Config, ConfigError};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub logging: LoggingSettings,
    pub api: ApiSettings,
    pub rpc: RpcSettings,
    pub aggregation: AggregationSettings,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AggregationSettings {
    Python(PythonAggregatorSettings),
}

#[derive(Debug, Deserialize, Clone)]
pub struct PythonAggregatorSettings {
    pub module: String,
    pub class: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiSettings {
    pub bind_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RpcSettings {
    pub bind_address: String,
    pub coordinator_address: String,
}

impl Settings {
    pub fn new(path: &str) -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(config::File::with_name(path))?;
        s.try_into()
    }
}
