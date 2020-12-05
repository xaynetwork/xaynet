use chrono::{DateTime, Utc};
use influxdb::InfluxDbWriteable;

/// An enum that contains all supported measurements.
pub enum Measurement {
    RoundParamSum,
    RoundParamUpdate,
    Phase,
    MasksTotalNumber,
    RoundTotalNumber,
    MessageSum,
    MessageUpdate,
    MessageSum2,
    MessageDiscarded,
    MessageRejected,
}

impl From<&Measurement> for &'static str {
    fn from(measurement: &Measurement) -> &'static str {
        match measurement {
            Measurement::RoundParamSum => "round_param_sum",
            Measurement::RoundParamUpdate => "round_param_update",
            Measurement::Phase => "phase",
            Measurement::MasksTotalNumber => "masks_total_number",
            Measurement::RoundTotalNumber => "round_total_number",
            Measurement::MessageSum => "message_sum",
            Measurement::MessageUpdate => "message_update",
            Measurement::MessageSum2 => "message_sum2",
            Measurement::MessageDiscarded => "message_discarded",
            Measurement::MessageRejected => "message_rejected",
        }
    }
}

impl ToString for Measurement {
    fn to_string(&self) -> String {
        Into::<&str>::into(self).into()
    }
}

/// A container that contains the tags of a metric.
pub struct Tags(Vec<(String, influxdb::Type)>);

impl Tags {
    /// Creates a new empty container for tags.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Adds a tag to the metric.
    pub fn add<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<influxdb::Type>,
    {
        self.0.push((key.into(), value.into()))
    }

    pub(in crate::metrics) fn into_inner(self) -> Vec<(String, influxdb::Type)> {
        self.0
    }
}

impl Default for Tags {
    fn default() -> Self {
        Self::new()
    }
}

/// A metrics data point.
pub(in crate::metrics) struct Metric {
    name: Measurement,
    time: DateTime<Utc>,
    value: influxdb::Type,
    tags: Option<Tags>,
}

impl Metric {
    pub(in crate::metrics) fn new(measurement: Measurement, value: influxdb::Type) -> Self {
        Self {
            name: measurement,
            time: Utc::now(),
            value,
            tags: None,
        }
    }

    pub(in crate::metrics) fn with_tags(&mut self, tags: Tags) {
        // we don't want to extend tags. The user can't call that method
        // multiple times because it is hidden behind the macro
        self.tags = Some(tags)
    }

    pub(in crate::metrics) fn into_query(self) -> influxdb::WriteQuery {
        let timestamp: ::influxdb::Timestamp = self.time.into();
        let mut query = timestamp.into_query(self.name.to_string());
        query = query.add_field("value", self.value);

        if let Some(tags) = self.tags {
            for (tag, value) in tags.into_inner() {
                query = query.add_tag(tag, value);
            }
        }

        query
    }
}

/// An event data point.
pub(in crate::metrics) struct Event {
    name: &'static str,
    time: DateTime<Utc>,
    title: String,
    description: Option<String>,
    tags: Option<String>,
}

impl Event {
    pub(in crate::metrics) fn new<T: Into<String>>(title: T) -> Self {
        Self {
            name: "events",
            time: Utc::now(),
            title: title.into(),
            description: None,
            tags: None,
        }
    }

    pub(in crate::metrics) fn with_description<D: Into<String>>(&mut self, description: D) {
        self.description = Some(description.into())
    }

    pub(in crate::metrics) fn with_tags(&mut self, tags: &[&str]) {
        // we don't want to extend tags. The user can't call that method
        // multiple times because it is hidden behind the macro
        self.tags = Some(tags.join(","))
    }

    pub(in crate::metrics) fn into_query(self) -> influxdb::WriteQuery {
        let timestamp: influxdb::Timestamp = self.time.into();
        let mut query = timestamp.into_query(self.name);
        query = query.add_field("title", self.title);

        if let Some(description) = self.description {
            query = query.add_field("description", description);
        }

        if let Some(tags) = self.tags {
            query = query.add_field("tags", tags);
        }

        query
    }
}
