use std::{borrow::Borrow, iter::IntoIterator};

use chrono::{DateTime, Utc};
use influxdb::{InfluxDbWriteable, Timestamp, Type, WriteQuery};

/// An enum that contains all supported measurements.
pub enum Measurement {
    RoundParamSum,
    RoundParamUpdate,
    Phase,
    MasksTotalNumber,
    RoundTotalNumber,
    MessageAccepted,
    MessageDiscarded,
    MessageRejected,
}

impl From<Measurement> for &'static str {
    fn from(measurement: Measurement) -> &'static str {
        match measurement {
            Measurement::RoundParamSum => "round_param_sum",
            Measurement::RoundParamUpdate => "round_param_update",
            Measurement::Phase => "phase",
            Measurement::MasksTotalNumber => "masks_total_number",
            Measurement::RoundTotalNumber => "round_total_number",
            Measurement::MessageAccepted => "message_accepted",
            Measurement::MessageDiscarded => "message_discarded",
            Measurement::MessageRejected => "message_rejected",
        }
    }
}

impl From<Measurement> for String {
    fn from(measurement: Measurement) -> Self {
        <&str>::from(measurement).into()
    }
}

/// A container that contains the tags of a metric.
pub struct Tags(Vec<(String, Type)>);

impl Tags {
    /// Creates a new empty container for tags.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Adds a tag to the metric.
    pub fn add(&mut self, tag: impl Into<String>, value: impl Into<Type>) {
        self.0.push((tag.into(), value.into()))
    }
}

impl Default for Tags {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for Tags {
    type Item = <Vec<(String, Type)> as IntoIterator>::Item;
    type IntoIter = <Vec<(String, Type)> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// A metrics data point.
pub(in crate::metrics) struct Metric {
    name: Measurement,
    time: DateTime<Utc>,
    value: Type,
    tags: Option<Tags>,
}

impl Metric {
    pub(in crate::metrics) fn new(measurement: Measurement, value: impl Into<Type>) -> Self {
        Self {
            name: measurement,
            time: Utc::now(),
            value: value.into(),
            tags: None,
        }
    }

    pub(in crate::metrics) fn with_tags<T, I>(mut self, tags: T) -> Self
    where
        T: Into<Option<I>>,
        I: Into<Tags>,
    {
        // It is by design that this function should only be called once.
        // see `Recorder::metric`
        // Therefore, we don't cover the case where we would extend `self.tags`
        // when `self.tags` already contains tags.
        self.tags = tags.into().map(Into::into);
        self
    }
}

impl From<Metric> for WriteQuery {
    fn from(metric: Metric) -> Self {
        let mut query = Timestamp::from(metric.time).into_query(metric.name);
        query = query.add_field("value", metric.value);

        if let Some(tags) = metric.tags {
            for (tag, value) in tags {
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
    pub(in crate::metrics) fn new(title: impl Into<String>) -> Self {
        Self {
            name: "event",
            time: Utc::now(),
            title: title.into(),
            description: None,
            tags: None,
        }
    }

    pub(in crate::metrics) fn with_description<D, S>(mut self, description: D) -> Self
    where
        D: Into<Option<S>>,
        S: Into<String>,
    {
        self.description = description.into().map(Into::into);
        self
    }

    pub(in crate::metrics) fn with_tags<T, A, B>(mut self, tags: T) -> Self
    where
        T: Into<Option<A>>,
        A: AsRef<[B]>,
        B: Borrow<str>,
    {
        // It is by design that this function should only be called once.
        // see `Recorder::metric`
        // Therefore, we don't cover the case where we would extend `self.tags`
        // when `self.tags` already contains tags.
        self.tags = tags.into().map(|tags| tags.as_ref().join(","));
        self
    }
}

impl From<Event> for WriteQuery {
    fn from(event: Event) -> Self {
        let mut query = Timestamp::from(event.time).into_query(event.name);
        query = query.add_field("title", event.title);

        if let Some(description) = event.description {
            query = query.add_field("description", description);
        }

        if let Some(tags) = event.tags {
            query = query.add_field("tags", tags);
        }

        query
    }
}

#[cfg(test)]
mod tests {
    use influxdb::Query;

    use super::*;

    /// Creates key-value tags for metrics.
    macro_rules! tags {
        ($(($tag: expr, $val: expr)),+ $(,)?) => {
            {
                let mut tags = crate::metrics::Tags::new();
                $(
                    tags.add($tag, $val);
                )+
                tags
            }
        };
    }

    #[test]
    fn test_basic_metric() {
        let metric = Metric::new(Measurement::Phase, 1);
        assert!(WriteQuery::from(metric)
            .build()
            .unwrap()
            .get()
            .starts_with("phase value=1i "))
    }

    #[test]
    fn test_metric_with_tag() {
        let metric = Metric::new(Measurement::Phase, 1).with_tags(tags![("key", 42)]);
        assert!(WriteQuery::from(metric)
            .build()
            .unwrap()
            .get()
            .starts_with("phase,key=42 value=1i "))
    }

    #[test]
    fn test_metric_with_tags() {
        let metric = Metric::new(Measurement::Phase, 1).with_tags(tags![
            ("key_1", 42),
            ("key_2", "42"),
            ("key_3", 1.0f32),
        ]);
        assert!(WriteQuery::from(metric)
            .build()
            .unwrap()
            .get()
            .starts_with("phase,key_1=42,key_2=42,key_3=1 value=1i "))
    }

    #[test]
    fn test_basic_event() {
        let event = Event::new("error");
        assert!(WriteQuery::from(event)
            .build()
            .unwrap()
            .get()
            .starts_with("event title=\"error\" "))
    }

    #[test]
    fn test_event_with_description() {
        let event = Event::new("error").with_description("description");
        assert!(WriteQuery::from(event)
            .build()
            .unwrap()
            .get()
            .starts_with("event title=\"error\",description=\"description\" "))
    }

    #[test]
    fn test_event_with_description_and_tag() {
        let event = Event::new("error")
            .with_description("description")
            .with_tags(["tag"]);
        assert!(WriteQuery::from(event)
            .build()
            .unwrap()
            .get()
            .starts_with("event title=\"error\",description=\"description\",tags=\"tag\" "))
    }

    #[test]
    fn test_event_with_description_and_tags() {
        let event = Event::new("error")
            .with_description("description")
            .with_tags(["tag_1", "tag_2"]);
        assert!(WriteQuery::from(event)
            .build()
            .unwrap()
            .get()
            .starts_with("event title=\"error\",description=\"description\",tags=\"tag_1,tag_2\" "))
    }

    #[test]
    fn test_event_with_tag() {
        let event = Event::new("error").with_tags(["tag"]);
        assert!(WriteQuery::from(event)
            .build()
            .unwrap()
            .get()
            .starts_with("event title=\"error\",tags=\"tag\" "))
    }
}
