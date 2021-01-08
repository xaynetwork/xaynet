use crate::{
    data_combination::data_points::data_point::{CalculateDataPoints, DataPointMetadata},
    data_provision::analytics_event::AnalyticsEvent,
};

// TODO: accept an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
pub struct WasActivePastNDays {
    metadata: DataPointMetadata,
    events: Vec<AnalyticsEvent>,
}

impl WasActivePastNDays {
    pub fn new(metadata: DataPointMetadata, events: Vec<AnalyticsEvent>) -> WasActivePastNDays {
        WasActivePastNDays { metadata, events }
    }
}

impl CalculateDataPoints for WasActivePastNDays {
    fn metadata(&self) -> DataPointMetadata {
        self.metadata
    }

    fn calculate(&self) -> Vec<u32> {
        let value = self.events.is_empty();
        if value == true {
            vec![0]
        } else {
            vec![1]
        }
    }
}
