use crate::data_combiner::data_points::data_point::{Calculate, DataPointMetadata};
use crate::repo::analytics_event::AnalyticsEvent;

pub struct WasActivePastNDays {
    metadata: DataPointMetadata,
    events: Vec<AnalyticsEvent>,
}

impl WasActivePastNDays {
    pub fn new(metadata: DataPointMetadata, events: Vec<AnalyticsEvent>) -> WasActivePastNDays {
        WasActivePastNDays { metadata, events }
    }
}

impl Calculate for WasActivePastNDays {
    fn metadata(&self) -> DataPointMetadata {
        self.metadata
    }

    fn calculate(&self) -> Vec<u32> {
        let value = !self.events.is_empty();
        if value == false {
            vec![0]
        } else {
            vec![1]
        }
    }
}
