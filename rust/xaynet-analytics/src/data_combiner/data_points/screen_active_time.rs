use chrono::Duration;

use crate::{
    data_combiner::data_points::data_point::{CalculateDataPoints, DataPointMetadata},
    repo::analytics_event::{AnalyticsEvent, AnalyticsEventType},
};

// TODO: accept an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
pub struct ScreenActiveTime {
    metadata: DataPointMetadata,
    events: Vec<AnalyticsEvent>,
}

impl ScreenActiveTime {
    pub fn new(metadata: DataPointMetadata, events: Vec<AnalyticsEvent>) -> ScreenActiveTime {
        ScreenActiveTime { metadata, events }
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_screen_and_app_events(&self) -> Vec<AnalyticsEvent> {
        self.events
            .iter()
            .filter(|event| {
                matches!(
                    event.event_type,
                    AnalyticsEventType::ScreenEnter | AnalyticsEventType::AppEvent
                )
            })
            .cloned()
            .collect()
    }
}

impl CalculateDataPoints for ScreenActiveTime {
    fn metadata(&self) -> DataPointMetadata {
        self.metadata
    }

    fn calculate(&self) -> Vec<u32> {
        let screen_and_app_events = self.get_screen_and_app_events();
        let value = screen_and_app_events
            .iter()
            .scan(
                screen_and_app_events.first().unwrap().timestamp,
                |last_timestamp, event| {
                    let duration = if event.event_type == AnalyticsEventType::ScreenEnter {
                        event.timestamp - *last_timestamp
                    } else {
                        Duration::zero()
                    };
                    *last_timestamp = event.timestamp;
                    Some(duration)
                }
            )
            .map(|duration| duration.num_milliseconds() as u32)
            .sum();
        vec![value]
    }
}
