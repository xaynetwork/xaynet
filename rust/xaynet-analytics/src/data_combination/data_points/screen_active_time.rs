use chrono::Duration;

use crate::{
    data_combination::data_points::data_point::{
        CalcScreenActiveTime,
        CalculateDataPoints,
        DataPointMetadata,
    },
    data_provision::analytics_event::{AnalyticsEvent, AnalyticsEventType},
};

impl CalcScreenActiveTime {
    pub fn new(metadata: DataPointMetadata, events: Vec<AnalyticsEvent>) -> CalcScreenActiveTime {
        CalcScreenActiveTime { metadata, events }
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

impl CalculateDataPoints for CalcScreenActiveTime {
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
                },
            )
            .map(|duration| duration.num_milliseconds() as u32)
            .sum();
        vec![value]
    }
}
