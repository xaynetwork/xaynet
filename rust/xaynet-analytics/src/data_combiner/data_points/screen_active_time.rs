use chrono::Duration;

use crate::data_combiner::data_points::data_point::{CalculateDataPoints, DataPointMetadata};
use crate::repo::analytics_event::{AnalyticsEvent, AnalyticsEventType};

pub struct ScreenActiveTime {
    metadata: DataPointMetadata,
    events: Vec<AnalyticsEvent>,
}

impl ScreenActiveTime {
    pub fn new(metadata: DataPointMetadata, events: Vec<AnalyticsEvent>) -> ScreenActiveTime {
        ScreenActiveTime { metadata, events }
    }

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

    fn calculate_duration_between_events(&self) -> Vec<Duration> {
        let mut duration_between_events: Vec<Duration> = Vec::new();
        let screen_and_app_events = self.get_screen_and_app_events();
        for this_event in screen_and_app_events.iter() {
            let has_next = this_event != screen_and_app_events.last().unwrap();
            if has_next && this_event.event_type == AnalyticsEventType::ScreenEnter {
                let mut peekable_events = screen_and_app_events.clone().into_iter().peekable();
                let next_event = peekable_events.peek().unwrap();
                let duration = next_event.timestamp - this_event.timestamp;
                duration_between_events.push(duration);
            }
        }
        duration_between_events
    }
}

impl CalculateDataPoints for ScreenActiveTime {
    fn metadata(&self) -> DataPointMetadata {
        self.metadata
    }

    fn calculate(&self) -> Vec<u32> {
        let duration_between_events = self.calculate_duration_between_events();
        let durations_in_milliseconds: Vec<u32> = duration_between_events
            .into_iter()
            .map(|duration| duration.num_milliseconds() as u32)
            .collect();
        let value = durations_in_milliseconds.into_iter().sum();
        vec![value]
    }
}
