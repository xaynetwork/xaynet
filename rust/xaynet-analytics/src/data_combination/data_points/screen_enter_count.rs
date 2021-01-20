use crate::{
    data_combination::data_points::data_point::{
        CalcScreenEnterCount,
        CalculateDataPoints,
        DataPointMetadata,
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

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Duration, Utc};

    use super::*;
    use crate::data_combination::data_points::data_point::{Period, PeriodUnit};

    #[test]
    fn test_calculate_when_no_events() {
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), Utc::now());
        let screen_enter_count = CalcScreenEnterCount::new(metadata, Vec::new());
        assert_eq!(screen_enter_count.calculate(), vec![0]);
    }

    #[test]
    fn test_calculate_when_one_event() {
        let end_period = DateTime::parse_from_rfc3339("2021-01-01T01:01:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        let events = vec![AnalyticsEvent::new(
            "test1",
            AnalyticsEventType::ScreenEnter,
            end_period - Duration::hours(12),
            "screen".to_string(),
        )];
        let screen_enter_count = CalcScreenEnterCount::new(metadata, events);
        assert_eq!(screen_enter_count.calculate(), vec![1]);
    }

    #[test]
    fn test_calculate_when_two_events() {
        let end_period = DateTime::parse_from_rfc3339("2021-02-02T02:02:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        let screen_route = "home_screen".to_string();
        let events = vec![
            AnalyticsEvent::new(
                "test1",
                AnalyticsEventType::ScreenEnter,
                end_period - Duration::hours(9),
                screen_route.clone(),
            ),
            AnalyticsEvent::new(
                "test2",
                AnalyticsEventType::ScreenEnter,
                end_period - Duration::hours(18),
                screen_route,
            ),
        ];
        let screen_enter_count = CalcScreenEnterCount::new(metadata, events);
        assert_eq!(screen_enter_count.calculate(), vec![2]);
    }
}
