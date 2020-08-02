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
pub struct Sum {
    pub time: DateTime<Utc>,
    pub sum: f64,
}

#[derive(InfluxDbWriteable)]
pub struct Update {
    pub time: DateTime<Utc>,
    pub update: f64,
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
    pub sum: u8,
    #[tag]
    pub round_id: u64,
    #[tag]
    pub phase: u8,
}

#[derive(InfluxDbWriteable)]
pub struct MessageUpdate {
    pub time: DateTime<Utc>,
    pub update: u8,
    #[tag]
    pub round_id: u64,
    #[tag]
    pub phase: u8,
}

#[derive(InfluxDbWriteable)]
pub struct MessageSum2 {
    pub time: DateTime<Utc>,
    pub sum2: u8,
    #[tag]
    pub round_id: u64,
    #[tag]
    pub phase: u8,
}

#[derive(InfluxDbWriteable)]
pub struct MessageDiscarded {
    pub time: DateTime<Utc>,
    pub discarded: u8,
    #[tag]
    pub round_id: u64,
    #[tag]
    pub phase: u8,
}

#[derive(InfluxDbWriteable)]
pub struct MessageRejected {
    pub time: DateTime<Utc>,
    pub rejected: u8,
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
