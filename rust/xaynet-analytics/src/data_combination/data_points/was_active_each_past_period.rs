use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::{
    data_combination::data_points::data_point::{
        CalcWasActiveEachPastPeriod,
        CalculateDataPoints,
        DataPointMetadata,
    },
    data_provision::analytics_event::AnalyticsEvent,
};

impl CalcWasActiveEachPastPeriod {
    pub fn new(
        metadata: DataPointMetadata,
        events: Vec<AnalyticsEvent>,
        period_thresholds: Vec<DateTime<Utc>>,
    ) -> CalcWasActiveEachPastPeriod {
        CalcWasActiveEachPastPeriod {
            metadata,
            events,
            period_thresholds,
        }
    }

    // TODO: this could possibly later be optimised by sorting events by timestamp and caching the last timestamp added to the HashMap
    fn group_timestamps_by_period_threshold(&self) -> HashMap<DateTime<Utc>, Vec<DateTime<Utc>>> {
        let mut timestamps_by_period_threshold = HashMap::new();
        for these_thresholds in self.period_thresholds.windows(2) {
            // safe unwrap: `windows` guarantees that there are at least two elements.
            // If `period_thresholds` contains less than two elements, this code block is not executed
            let newer_threshold = these_thresholds.first().unwrap();
            let older_threshold = these_thresholds.last().unwrap();
            let timestamps: Vec<DateTime<Utc>> = self
                .events
                .iter()
                .filter(|event| {
                    event.timestamp < *newer_threshold && event.timestamp > *older_threshold
                })
                .map(|event| event.timestamp)
                .collect();
            timestamps_by_period_threshold.insert(*newer_threshold, timestamps);
        }
        timestamps_by_period_threshold
    }
}

impl CalculateDataPoints for CalcWasActiveEachPastPeriod {
    fn metadata(&self) -> DataPointMetadata {
        self.metadata
    }

    fn calculate(&self) -> Vec<u32> {
        let timestamps_by_period_threshold = self.group_timestamps_by_period_threshold();
        timestamps_by_period_threshold
            .values()
            .map(|timestamps| !timestamps.is_empty() as u32)
            .collect::<Vec<u32>>()
    }
}
