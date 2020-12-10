use crate::data_combiner::data_points::data_point::{Calculate, DataPointMetadata};
use crate::repo::analytics_event::{AnalyticsEvent, AnalyticsEventType};

pub struct ScreenEnterCount {
    metadata: DataPointMetadata,
    events: Vec<AnalyticsEvent>,
}

impl ScreenEnterCount {
    pub fn new(metadata: DataPointMetadata, events: Vec<AnalyticsEvent>) -> ScreenEnterCount {
        ScreenEnterCount { metadata, events }
    }
}

impl Calculate for ScreenEnterCount {
    fn metadata(&self) -> DataPointMetadata {
        self.metadata
    }

    fn calculate(&self) -> Vec<u32> {
        let value = self
            .events
            .clone()
            .into_iter()
            .filter(|event| event.event_type == AnalyticsEventType::ScreenEnter)
            .count() as u32;
        vec![value]
    }
}
