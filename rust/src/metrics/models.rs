use chrono::{DateTime, Utc};
use influxdb::InfluxDbWriteable;

pub enum Measurement {
    StateMachine,
    Event,
}

impl From<&Measurement> for &'static str {
    fn from(measurement: &Measurement) -> &'static str {
        match measurement {
            Measurement::StateMachine => "state_machine",
            Measurement::Event => "event",
        }
    }
}

impl ToString for Measurement {
    fn to_string(&self) -> String {
        Into::<&str>::into(self).into()
    }
}

#[derive(InfluxDbWriteable)]
pub struct RoundParamSum {
    pub time: DateTime<Utc>,
    pub round_param_sum: f64,
}

#[derive(InfluxDbWriteable)]
pub struct RoundParamUpdate {
    pub time: DateTime<Utc>,
    pub round_param_update: f64,
}

#[derive(InfluxDbWriteable)]
pub struct Phase {
    pub time: DateTime<Utc>,
    pub phase: u8,
}

#[derive(InfluxDbWriteable)]
pub struct MasksTotalNumber {
    pub time: DateTime<Utc>,
    pub masks_total_number: u64,
}

#[derive(InfluxDbWriteable)]
pub struct RoundTotalNumber {
    pub time: DateTime<Utc>,
    pub round_total_number: u64,
}

#[derive(InfluxDbWriteable)]
pub struct RoundSuccessful {
    pub time: DateTime<Utc>,
    pub round_successful: u8,
}

#[derive(InfluxDbWriteable)]
pub struct MessageSum {
    pub time: DateTime<Utc>,
    pub message_sum: u8,
    #[tag]
    pub round_id: u64,
    #[tag]
    pub phase: u8,
}

#[derive(InfluxDbWriteable)]
pub struct MessageUpdate {
    pub time: DateTime<Utc>,
    pub message_update: u8,
    #[tag]
    pub round_id: u64,
    #[tag]
    pub phase: u8,
}

#[derive(InfluxDbWriteable)]
pub struct MessageSum2 {
    pub time: DateTime<Utc>,
    pub message_sum2: u8,
    #[tag]
    pub round_id: u64,
    #[tag]
    pub phase: u8,
}

#[derive(InfluxDbWriteable)]
pub struct MessageDiscarded {
    pub time: DateTime<Utc>,
    pub message_discarded: u8,
    #[tag]
    pub round_id: u64,
    #[tag]
    pub phase: u8,
}

#[derive(InfluxDbWriteable)]
pub struct MessageRejected {
    pub time: DateTime<Utc>,
    pub message_rejected: u8,
    #[tag]
    pub round_id: u64,
    #[tag]
    pub phase: u8,
}

#[derive(InfluxDbWriteable)]
pub struct Event {
    pub time: DateTime<Utc>,
    pub title: String,
    pub text: Option<String>,
    pub tags: Option<String>,
}
