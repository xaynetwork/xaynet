use crate::{
    data_combination::data_points::data_point::{
        CalcScreenEnterCount, CalculateDataPoints, DataPointMetadata,
    },
    data_provision::analytics_event::{AnalyticsEvent, AnalyticsEventType},
};

impl CalcScreenEnterCount {
    pub fn new(metadata: DataPointMetadata, events: Vec<AnalyticsEvent>) -> CalcScreenEnterCount {
        CalcScreenEnterCount { metadata, events }
    }
}

impl CalculateDataPoints for CalcScreenEnterCount {
    fn metadata(&self) -> DataPointMetadata {
        self.metadata
    }

    fn calculate(&self) -> Vec<u32> {
        let value = self
            .events
            .iter()
            .filter(|event| event.event_type == AnalyticsEventType::ScreenEnter)
            .count() as u32;
        vec![value]
    }
}
