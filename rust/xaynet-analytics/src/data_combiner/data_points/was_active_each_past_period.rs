use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::{
    data_combiner::data_points::data_point::{CalculateDataPoints, DataPointMetadata},
    repo::analytics_event::AnalyticsEvent,
};

// TODO: accept an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
pub struct WasActiveEachPastPeriod {
    metadata: DataPointMetadata,
    events: Vec<AnalyticsEvent>,
    period_thresholds: Vec<DateTime<Utc>>,
}

impl WasActiveEachPastPeriod {
    pub fn new(
        metadata: DataPointMetadata,
        events: Vec<AnalyticsEvent>,
        period_thresholds: Vec<DateTime<Utc>>,
    ) -> WasActiveEachPastPeriod {
        WasActiveEachPastPeriod {
            metadata,
            events,
            period_thresholds,
        }
    }

    // TODO: this could possibly later be optimised by sorting events by timestamp and caching the last timestamp added to the HashMap
    fn group_timestamps_by_period_threshold(&self) -> HashMap<DateTime<Utc>, Vec<DateTime<Utc>>> {
        let mut timestamps_by_period_threshold = HashMap::new();
        for these_thresholds in self.period_thresholds.windows(2) {
            let newer_threshold = these_thresholds.first().unwrap();
            let older_threshold = these_thresholds.last().unwrap();
            for event in self.events.iter() {
                if event.timestamp < *newer_threshold && event.timestamp > *older_threshold {
                    timestamps_by_period_threshold
                        .entry(*newer_threshold)
                        .or_insert(Vec::new())
                        .push(event.timestamp);
                }
            }
        }
        timestamps_by_period_threshold
    }
}

impl CalculateDataPoints for WasActiveEachPastPeriod {
    fn metadata(&self) -> DataPointMetadata {
        self.metadata
    }

    fn calculate(&self) -> Vec<u32> {
        let timestamps_by_period_threshold = self.group_timestamps_by_period_threshold();
        timestamps_by_period_threshold
            .values()
            .map(|timestamps| !timestamps.is_empty())
            .map(|was_active| if was_active == false { 0 } else { 1 })
            .collect::<Vec<u32>>()
    }
}
