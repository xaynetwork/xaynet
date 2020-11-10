//! Utilities for sending metrics to an InfluxDB instance.
//!
//! ## Basic usage:
//!
//! ```compile_fail
//! async fn main() {
//!     let (metrics_service, metrics_sender) =
//!         MetricsService::new("http://127.0.0.1:8086", "metrics");
//!     let metrics_service_handle =
//!         tokio::spawn(async { run_metric_service(metrics_service).await });
//!
//!     metrics_sender.send(metrics::phase::update(PhaseName::Idle));
//!
//!     // The metrics service is automatically resolved after all senders have been dropped
//!     // and all remaining messages have been processed.
//!     drop(metrics_sender);
//!     let _ = metrics_service_handle.await;
//! }
//! ```

mod models;

#[cfg(not(test))]
pub(crate) mod service;
#[cfg(not(test))]
pub use self::service::{run_metric_service, MetricsSender, MetricsService};
#[cfg(test)]
pub mod tests;
#[cfg(test)]
pub use self::tests::MetricsSender;

pub mod round_parameters {
    use super::models::{DataPoint, Measurement};
    use crate::state_machine::phases::PhaseName;
    use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
    pub mod sum {
        use super::*;

        /// Updates the measurement `round_param_sum` with the value of `sum`.
        ///
        /// Creates an influx data point with the following properties:
        ///
        /// | property    | value                    |
        /// |-------------|--------------------------|
        /// | measurement | `round_param_sum`        |
        /// | field_key   | `value`                  |
        /// | field_value | value of `sum`           |
        /// | tag_key     | `"round_id"`             |
        /// | tag_value   | value of `round_id`      |
        /// | tag_key     | `"phase"`                |
        /// | tag_value   | value of `phase` as `u8` |
        pub fn update(sum: f64, round_id: u64, phase: PhaseName) -> WriteQuery {
            DataPoint {
                time: Timestamp::Now.into(),
                value: sum,
                round_id: Some(round_id),
                phase: Some(phase as u8),
            }
            .into_query(Measurement::RoundParamSum.to_string())
        }
    }

    pub mod update {
        use super::*;

        /// Updates the measurement `round_param_update` with the value of `update`.
        ///
        /// Creates an influx data point with the following properties:
        ///
        /// | property    | value                    |
        /// |-------------|--------------------------|
        /// | measurement | `round_param_update`     |
        /// | field_key   | `value`                  |
        /// | field_value | value of `update`        |
        /// | tag_key     | `"round_id"`             |
        /// | tag_value   | value of `round_id`      |
        /// | tag_key     | `"phase"`                |
        /// | tag_value   | value of `phase` as `u8` |
        pub fn update(update: f64, round_id: u64, phase: PhaseName) -> WriteQuery {
            DataPoint {
                time: Timestamp::Now.into(),
                value: update,
                round_id: Some(round_id),
                phase: Some(phase as u8),
            }
            .into_query(Measurement::RoundParamUpdate.to_string())
        }
    }
}

pub mod phase {
    use super::models::{DataPoint, Event, Measurement};
    use crate::state_machine::phases::{PhaseName, PhaseStateError};
    use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
    pub mod error {
        use super::*;

        /// Emits the measurement `event` with the value of `error`.
        ///
        /// Creates an influx data point with the following properties:
        ///
        /// | property    | value                       |
        /// |-------------|-----------------------------|
        /// | measurement | `event             `        |
        /// | field_key   | `title`                     |
        /// | field_value | value of `error.to_string()`|
        pub fn emit(error: &PhaseStateError) -> WriteQuery {
            Event {
                time: Timestamp::Now.into(),
                title: error.to_string(),
                text: None,
                tags: None,
            }
            .into_query(Measurement::Event.to_string())
        }
    }

    /// Updates the measurement `phase` with the value of `phase`.
    ///
    /// Creates an influx data point with the following properties:
    ///
    /// | property    | value                    |
    /// |-------------|--------------------------|
    /// | measurement | `phase`                  |
    /// | field_key   | `value`                  |
    /// | field_value | value of `phase` as `u8` |
    pub fn update(phase: PhaseName) -> WriteQuery {
        DataPoint {
            time: Timestamp::Now.into(),
            value: phase as u8,
            round_id: None,
            phase: None,
        }
        .into_query(Measurement::Phase.to_string())
    }
}

pub mod masks {
    use super::models::{DataPoint, Measurement};
    use crate::state_machine::phases::PhaseName;
    use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
    pub mod total_number {
        use super::*;

        /// Updates the measurement `masks_total_number` with the value of `total_number`.
        ///
        /// Creates an influx data point with the following properties:
        ///
        /// | property    | value                    |
        /// |-------------|--------------------------|
        /// | measurement | `masks_total_number`     |
        /// | field_key   | `value`                  |
        /// | field_value | value of `total_number`  |
        /// | tag_key     | `"round_id"`             |
        /// | tag_value   | value of `round_id`      |
        /// | tag_key     | `"phase"`                |
        /// | tag_value   | value of `phase` as `u8` |
        pub fn update(total_number: u64, round_id: u64, phase: PhaseName) -> WriteQuery {
            DataPoint {
                time: Timestamp::Now.into(),
                value: total_number,
                round_id: Some(round_id),
                phase: Some(phase as u8),
            }
            .into_query(Measurement::MasksTotalNumber.to_string())
        }
    }
}

pub mod round {
    use super::models::{DataPoint, Measurement};
    use crate::state_machine::phases::PhaseName;
    use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
    pub mod total_number {
        use super::*;

        /// Updates the measurement `round_total_number` with the value of `total_number`.
        ///
        /// Creates an influx data point with the following properties:
        ///
        /// | property    | value                    |
        /// |-------------|--------------------------|
        /// | measurement | `round_total_number`     |
        /// | field_key   | `value`                  |
        /// | field_value | value of `total_number`  |
        pub fn update(total_number: u64) -> WriteQuery {
            DataPoint {
                time: Timestamp::Now.into(),
                value: total_number,
                round_id: None,
                phase: None,
            }
            .into_query(Measurement::RoundTotalNumber.to_string())
        }
    }

    pub mod successful {
        use super::*;

        /// Increments value of the measurement `round_successful` by `1`.
        ///
        /// Creates an influx data point with the following properties:
        ///
        /// | property    | value                    |
        /// |-------------|--------------------------|
        /// | measurement | `round_successful`       |
        /// | field_key   | `value`                  |
        /// | field_value | `1`                      |
        /// | tag_key     | `"round_id"`             |
        /// | tag_value   | value of `round_id`      |
        /// | tag_key     | `"phase"`                |
        /// | tag_value   | value of `phase` as `u8` |
        pub fn increment(round_id: u64, phase: PhaseName) -> WriteQuery {
            DataPoint {
                time: Timestamp::Now.into(),
                value: 1,
                round_id: Some(round_id),
                phase: Some(phase as u8),
            }
            .into_query(Measurement::RoundSuccessful.to_string())
        }
    }
}

pub mod message {
    use super::models::{DataPoint, Measurement};
    use crate::state_machine::phases::PhaseName;
    use influxdb::{InfluxDbWriteable, Timestamp, WriteQuery};
    pub mod sum {
        use super::*;

        /// Increments value of the measurement `message_sum` by `1`.
        ///
        /// Creates an influx data point with the following properties:
        ///
        /// | property    | value                    |
        /// |-------------|--------------------------|
        /// | measurement | `message_sum`            |
        /// | field_key   | `value`                  |
        /// | field_value | `1`                      |
        /// | tag_key     | `"round_id"`             |
        /// | tag_value   | value of `round_id`      |
        /// | tag_key     | `"phase"`                |
        /// | tag_value   | value of `phase` as `u8` |
        pub fn increment(round_id: u64, phase: PhaseName) -> WriteQuery {
            DataPoint {
                time: Timestamp::Now.into(),
                value: 1,
                round_id: Some(round_id),
                phase: Some(phase as u8),
            }
            .into_query(Measurement::MessageSum.to_string())
        }
    }

    pub mod update {
        use super::*;

        /// Increments value of the measurement `message_update` by `1`.
        ///
        /// Creates an influx data point with the following properties:
        ///
        /// | property    | value                    |
        /// |-------------|--------------------------|
        /// | measurement | `message_update`         |
        /// | field_key   | `value`                  |
        /// | field_value | `1`                      |
        /// | tag_key     | `"round_id"`             |
        /// | tag_value   | value of `round_id`      |
        /// | tag_key     | `"phase"`                |
        /// | tag_value   | value of `phase` as `u8` |
        pub fn increment(round_id: u64, phase: PhaseName) -> WriteQuery {
            DataPoint {
                time: Timestamp::Now.into(),
                value: 1,
                round_id: Some(round_id),
                phase: Some(phase as u8),
            }
            .into_query(Measurement::MessageUpdate.to_string())
        }
    }

    pub mod sum2 {
        use super::*;

        /// Increments value of the measurement `message_sum2` by `1`.
        ///
        /// Creates an influx data point with the following properties:
        ///
        /// | property    | value                    |
        /// |-------------|--------------------------|
        /// | measurement | `message_sum2`           |
        /// | field_key   | `value`                  |
        /// | field_value | `1`                      |
        /// | tag_key     | `"round_id"`             |
        /// | tag_value   | value of `round_id`      |
        /// | tag_key     | `"phase"`                |
        /// | tag_value   | value of `phase` as `u8` |
        pub fn increment(round_id: u64, phase: PhaseName) -> WriteQuery {
            DataPoint {
                time: Timestamp::Now.into(),
                value: 1,
                round_id: Some(round_id),
                phase: Some(phase as u8),
            }
            .into_query(Measurement::MessageSum2.to_string())
        }
    }

    pub mod discarded {
        use super::*;

        /// Increments value of the measurement `message_discarded` by `1`.
        ///
        /// Creates an influx data point with the following properties:
        ///
        /// | property    | value                    |
        /// |-------------|--------------------------|
        /// | measurement | `message_discarded`      |
        /// | field_key   | `value`                  |
        /// | field_value | `1`                      |
        /// | tag_key     | `"round_id"`             |
        /// | tag_value   | value of `round_id`      |
        /// | tag_key     | `"phase"`                |
        /// | tag_value   | value of `phase` as `u8` |
        pub fn increment(round_id: u64, phase: PhaseName) -> WriteQuery {
            DataPoint {
                time: Timestamp::Now.into(),
                value: 1,
                round_id: Some(round_id),
                phase: Some(phase as u8),
            }
            .into_query(Measurement::MessageDiscarded.to_string())
        }
    }

    pub mod rejected {
        use super::*;

        /// Increments value of the measurement `message_rejected` by `1`.
        ///
        /// Creates an influx data point with the following properties:
        ///
        /// | property    | value                    |
        /// |-------------|--------------------------|
        /// | measurement | `message_rejected`       |
        /// | field_key   | `value`                  |
        /// | field_value | `1`                      |
        /// | tag_key     | `"round_id"`             |
        /// | tag_value   | value of `round_id`      |
        /// | tag_key     | `"phase"`                |
        /// | tag_value   | value of `phase` as `u8` |
        pub fn increment(round_id: u64, phase: PhaseName) -> WriteQuery {
            DataPoint {
                time: Timestamp::Now.into(),
                value: 1,
                round_id: Some(round_id),
                phase: Some(phase as u8),
            }
            .into_query(Measurement::MessageRejected.to_string())
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::state_machine::phases::{PhaseName, PhaseStateError, UnmaskStateError};
    use influxdb::Query;

    // The fields of the WriteQuery are private and there are no kinds of getters for the fields.
    // One way to get something is via `build`.
    // We are using `contains` because we don't to check the timestamp at the end.
    // e.g. "state_machine sum=0.6 1596463830315036000"

    #[test]
    fn test_round_parameters_sum() {
        let query = round_parameters::sum::update(0.6, 1, PhaseName::Sum).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("round_param_sum,round_id=\\\"1\\\",phase=\\\"1\\\" value=0.6"));
    }

    #[test]
    fn test_round_parameters_update() {
        let query = round_parameters::update::update(0.8, 1, PhaseName::Sum).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("round_param_update,round_id=\\\"1\\\",phase=\\\"1\\\" value=0.8"));
    }

    #[test]
    fn test_phase_name() {
        let query = phase::update(PhaseName::Idle).build();
        assert!(format!("{:?}", query.unwrap()).contains("phase value=0"));

        let query = phase::update(PhaseName::Sum).build();
        assert!(format!("{:?}", query.unwrap()).contains("phase value=1"));

        let query = phase::update(PhaseName::Update).build();
        assert!(format!("{:?}", query.unwrap()).contains("phase value=2"));

        let query = phase::update(PhaseName::Sum2).build();
        assert!(format!("{:?}", query.unwrap()).contains("phase value=3"));

        let query = phase::update(PhaseName::Unmask).build();
        assert!(format!("{:?}", query.unwrap()).contains("phase value=4"));

        let query = phase::update(PhaseName::Error).build();
        assert!(format!("{:?}", query.unwrap()).contains("phase value=5"));

        let query = phase::update(PhaseName::Shutdown).build();
        assert!(format!("{:?}", query.unwrap()).contains("phase value=6"));
    }

    #[test]
    fn test_phase_error() {
        let query = phase::error::emit(&PhaseStateError::Unmask(UnmaskStateError::NoMask)).build();
        assert!(format!("{:?}", query.unwrap()).contains(
            "event title=\\\"unmask\\\\ global\\\\ model\\\\ error:\\\\ no\\\\ mask\\\\ found\\\""
        ));
    }

    #[test]
    fn test_masks_total_number() {
        let query = masks::total_number::update(12, 1, PhaseName::Sum).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("masks_total_number,round_id=\\\"1\\\",phase=\\\"1\\\" value=12"));
    }

    #[test]
    fn test_round_total_number() {
        let query = round::total_number::update(2).build();
        assert!(format!("{:?}", query.unwrap()).contains("round_total_number value=2"));
    }

    #[test]
    fn test_round_successful() {
        let query = round::successful::increment(1, PhaseName::Sum).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("round_successful,round_id=\\\"1\\\",phase=\\\"1\\\" value=1"));
    }

    #[test]
    fn test_message_sum() {
        let query = message::sum::increment(1, PhaseName::Sum).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("message_sum,round_id=\\\"1\\\",phase=\\\"1\\\" value=1"));
    }

    #[test]
    fn test_message_update() {
        let query = message::update::increment(1, PhaseName::Update).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("message_update,round_id=\\\"1\\\",phase=\\\"2\\\" value=1"));
    }

    #[test]
    fn test_message_sum2() {
        let query = message::sum2::increment(1, PhaseName::Sum2).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("message_sum2,round_id=\\\"1\\\",phase=\\\"3\\\" value=1"));
    }

    #[test]
    fn test_message_discarded() {
        let query = message::discarded::increment(1, PhaseName::Idle).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("message_discarded,round_id=\\\"1\\\",phase=\\\"0\\\" value=1"));
    }

    #[test]
    fn test_message_rejected() {
        let query = message::rejected::increment(1, PhaseName::Sum).build();
        assert!(format!("{:?}", query.unwrap())
            .contains("message_rejected,round_id=\\\"1\\\",phase=\\\"1\\\" value=1"));
    }
}
