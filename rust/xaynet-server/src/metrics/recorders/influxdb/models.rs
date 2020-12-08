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
        // It is by design that this function should only be called once.
        // see `Recorder::metric`
        // Therefore, we don't cover the case where we would extend `self.tags`
        // when `self.tags` already contains tags.
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
            name: "event",
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
        // It is by design that this function should only be called once.
        // see `Recorder::metric`
        // Therefore, we don't cover the case where we would extend `self.tags`
        // when `self.tags` already contains tags.
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

#[cfg(test)]
mod tests {
    use super::*;
    use influxdb::Query;

    #[test]
    fn test_basic_metric() {
        let metric = Metric::new(Measurement::Phase, 1.into());
        assert!(metric
            .into_query()
            .build()
            .unwrap()
            .get()
            .starts_with("phase value=1i "))
    }

    #[test]
    fn test_metric_with_tag() {
        let mut metric = Metric::new(Measurement::Phase, 1.into());
        let mut tag = Tags::default();
        tag.add("key", 42);
        metric.with_tags(tag);
        assert!(metric
            .into_query()
            .build()
            .unwrap()
            .get()
            .starts_with("phase,key=42 value=1i "))
    }

    #[test]
    fn test_metric_with_tags() {
        let mut metric = Metric::new(Measurement::Phase, 1.into());
        let mut tag = Tags::new();
        tag.add("key_1", 42);
        tag.add("key_2", "42");
        tag.add("key_3", 1.0f32);
        metric.with_tags(tag);
        assert!(metric
            .into_query()
            .build()
            .unwrap()
            .get()
            .starts_with("phase,key_1=42,key_2=42,key_3=1 value=1i "))
    }

    #[test]
    fn test_basic_event() {
        let event = Event::new("error");
        assert!(event
            .into_query()
            .build()
            .unwrap()
            .get()
            .starts_with("event title=\"error\" "))
    }

    #[test]
    fn test_event_with_description() {
        let mut event = Event::new("error");
        event.with_description("description");
        assert!(event
            .into_query()
            .build()
            .unwrap()
            .get()
            .starts_with("event title=\"error\",description=\"description\" "))
    }

    #[test]
    fn test_event_with_description_and_tag() {
        let mut event = Event::new("error");
        event.with_description("description");
        event.with_tags(&["tag"]);
        assert!(event
            .into_query()
            .build()
            .unwrap()
            .get()
            .starts_with("event title=\"error\",description=\"description\",tags=\"tag\" "))
    }

    #[test]
    fn test_event_with_description_and_tags() {
        let mut event = Event::new("error");
        event.with_description("description");
        event.with_tags(&["tag_1", "tag_2"]);
        assert!(event
            .into_query()
            .build()
            .unwrap()
            .get()
            .starts_with("event title=\"error\",description=\"description\",tags=\"tag_1,tag_2\" "))
    }

    #[test]
    fn test_event_with_tag() {
        let mut event = Event::new("error");
        event.with_tags(&["tag"]);
        assert!(event
            .into_query()
            .build()
            .unwrap()
            .get()
            .starts_with("event title=\"error\",tags=\"tag\" "))
    }
}
