use tracing_subscriber::filter::LevelFilter;

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingSettings {
    pub telemetry: Option<TelemetrySettings>,
    #[serde(default)]
    pub level: LogLevel,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelemetrySettings {
    pub service_name: String,
    pub jaeger_endpoint: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Off,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl Into<LevelFilter> for LogLevel {
    fn into(self) -> LevelFilter {
        match self {
            Self::Trace => LevelFilter::TRACE,
            Self::Debug => LevelFilter::DEBUG,
            Self::Info => LevelFilter::INFO,
            Self::Warn => LevelFilter::WARN,
            Self::Error => LevelFilter::ERROR,
            Self::Off => LevelFilter::OFF,
        }
    }
}
