use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::UNIX_EPOCH;

use crate::data_combiner::data_points::data_point::{CalculateDataPoints, DataPointMetadata};
use crate::repo::analytics_event::AnalyticsEvent;

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
        for this_threshold in self.period_thresholds.iter() {
            let next_threshold = if this_threshold != self.period_thresholds.iter().last().unwrap()
            {
                **self.period_thresholds.iter().peekable().peek().unwrap()
            } else {
                UNIX_EPOCH.into()
            };
            for event in self.events.iter() {
                if event.timestamp < *this_threshold && event.timestamp > next_threshold {
                    timestamps_by_period_threshold
                        .entry(*this_threshold)
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
