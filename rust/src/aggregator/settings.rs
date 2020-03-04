use config::{Config, ConfigError};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub log_level: String,
    pub api: ApiSettings,
    pub rpc: RpcSettings,
    pub aggregation: AggregationSettings,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationSettings {
    Python(PythonAggregatorSettings),
}

#[derive(Debug, Deserialize)]
pub struct PythonAggregatorSettings {
    pub module: String,
    pub class: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiSettings {
    pub bind_address: String,
}

#[derive(Debug, Deserialize)]
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
