use crate::{
    data_combination::data_points::data_point::{
        CalcWasActivePastNDays,
        CalculateDataPoints,
        DataPointMetadata,
    },
    data_provision::analytics_event::AnalyticsEvent,
};

impl CalcWasActivePastNDays {
    pub fn new(metadata: DataPointMetadata, events: Vec<AnalyticsEvent>) -> CalcWasActivePastNDays {
        CalcWasActivePastNDays { metadata, events }
    }
}

impl CalculateDataPoints for CalcWasActivePastNDays {
    fn metadata(&self) -> DataPointMetadata {
        self.metadata
    }

    fn calculate(&self) -> Vec<u32> {
        vec![!self.events.is_empty() as u32]
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::{
        data_combination::data_points::{
            data_point::{CalculateDataPoints, DataPointMetadata, Period, PeriodUnit},
            was_active_past_n_days::CalcWasActivePastNDays,
        },
        data_provision::analytics_event::{AnalyticsEvent, AnalyticsEventType},
    };

    #[test]
    fn test_calculate_without_events() {
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), Utc::now());
        let was_active_past_n_days = CalcWasActivePastNDays::new(metadata, Vec::new());
        assert_eq!(was_active_past_n_days.calculate(), vec![0]);
    }

    #[test]
    fn test_calculate_with_events() {
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), Utc::now());
        let events = vec![AnalyticsEvent::new(
            "test1",
            AnalyticsEventType::AppEvent,
            metadata.end - Duration::hours(12),
            None,
        )];
        let was_active_past_n_days = CalcWasActivePastNDays::new(metadata, events);
        assert_eq!(was_active_past_n_days.calculate(), vec![1]);
    }
}
