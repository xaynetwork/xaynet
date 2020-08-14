use chrono::{DateTime, Utc};
use influxdb::InfluxDbWriteable;

/// An enum that contains all supported measurements.
pub(in crate::metrics) enum Measurement {
    RoundParamSum,
    RoundParamUpdate,
    Phase,
    MasksTotalNumber,
    RoundTotalNumber,
    RoundSuccessful,
    MessageSum,
    MessageUpdate,
    MessageSum2,
    MessageDiscarded,
    MessageRejected,
    Event,
}

impl From<&Measurement> for &'static str {
    fn from(measurement: &Measurement) -> &'static str {
        match measurement {
            Measurement::RoundParamSum => "round_param_sum",
            Measurement::RoundParamUpdate => "round_param_update",
            Measurement::Phase => "phase",
            Measurement::MasksTotalNumber => "masks_total_number",
            Measurement::RoundTotalNumber => "round_total_number",
            Measurement::RoundSuccessful => "round_successful",
            Measurement::MessageSum => "message_sum",
            Measurement::MessageUpdate => "message_update",
            Measurement::MessageSum2 => "message_sum2",
            Measurement::MessageDiscarded => "message_discarded",
            Measurement::MessageRejected => "message_rejected",
            Measurement::Event => "event",
        }
    }
}

impl ToString for Measurement {
    fn to_string(&self) -> String {
        Into::<&str>::into(self).into()
    }
}

/// A generic influx data point.
pub(in crate::metrics) struct DataPoint<T: Into<influxdb::Type>> {
    pub time: DateTime<Utc>,
    pub value: T,
    pub round_id: Option<u64>,
    pub phase: Option<u8>,
}

impl<T: Into<influxdb::Type>> InfluxDbWriteable for DataPoint<T> {
    fn into_query<I: Into<String>>(self, name: I) -> influxdb::WriteQuery {
        let timestamp: ::influxdb::Timestamp = self.time.into();
        let mut query = timestamp.into_query(name);
        query = query.add_field("value", self.value);
        query = query.add_tag("round_id", self.round_id);
        query = query.add_tag("phase", self.phase);
        query
    }
}

/// An `event` data point.
#[derive(InfluxDbWriteable)]
pub(in crate::metrics) struct Event {
    pub time: DateTime<Utc>,
    pub title: String,
    pub text: Option<String>,
    pub tags: Option<String>,
}
