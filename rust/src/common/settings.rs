use serde::de::{self, Deserializer, Visitor};
use std::fmt;
use tracing_subscriber::filter::EnvFilter;

#[derive(Debug, Deserialize)]
pub struct LoggingSettings {
    pub telemetry: Option<TelemetrySettings>,
    #[serde(default = "default_env_filter")]
    #[serde(deserialize_with = "deserialize_env_filter")]
    pub filter: EnvFilter,
}

fn default_env_filter() -> EnvFilter {
    EnvFilter::try_new("info").unwrap()
}

fn deserialize_env_filter<'de, D>(deserializer: D) -> Result<EnvFilter, D::Error>
where
    D: Deserializer<'de>,
{
    struct EnvFilterVisitor;

    impl<'de> Visitor<'de> for EnvFilterVisitor {
        type Value = EnvFilter;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a valid tracing filter directive: https://docs.rs/tracing-subscriber/0.2.3/tracing_subscriber/filter/struct.EnvFilter.html#directives")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            EnvFilter::try_new(value).map_err(E::custom)
        }
    }

    deserializer.deserialize_str(EnvFilterVisitor)
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelemetrySettings {
    pub service_name: String,
    pub jaeger_endpoint: String,
}
