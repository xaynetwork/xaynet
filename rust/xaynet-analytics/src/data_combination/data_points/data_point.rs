use chrono::{DateTime, Utc};

use crate::data_provision::analytics_event::AnalyticsEvent;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PeriodUnit {
    Days,
    Weeks,
    Months,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Period {
    pub unit: PeriodUnit,
    pub n: u32,
}

impl Period {
    pub fn new(unit: PeriodUnit, n: u32) -> Period {
        Period { unit, n }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DataPointMetadata {
    pub period: Period,
    pub end: DateTime<Utc>,
}

impl DataPointMetadata {
    pub fn new(period: Period, end: DateTime<Utc>) -> DataPointMetadata {
        DataPointMetadata { period, end }
    }
}

pub trait CalculateDataPoints {
    fn metadata(&self) -> DataPointMetadata;

    fn calculate(&self) -> Vec<u32>;
}

pub enum DataPoint {
    ScreenActiveTime(CalcScreenActiveTime),
    ScreenEnterCount(CalcScreenEnterCount),
    WasActiveEachPastPeriod(CalcWasActiveEachPastPeriod),
    WasActivePastNDays(CalcWasActivePastNDays),
}

#[allow(dead_code)]
// TODO: will be called when preparing the data to be sent to the coordinator
impl DataPoint {
    fn calculate(&self) -> Vec<u32> {
        match self {
            DataPoint::ScreenActiveTime(data) => data.calculate(),
            DataPoint::ScreenEnterCount(data) => data.calculate(),
            DataPoint::WasActiveEachPastPeriod(data) => data.calculate(),
            DataPoint::WasActivePastNDays(data) => data.calculate(),
        }
    }
}

// TODO: accept an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
pub struct CalcScreenActiveTime {
    pub metadata: DataPointMetadata,
    pub events: Vec<AnalyticsEvent>,
}

// TODO: accept an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
pub struct CalcScreenEnterCount {
    pub metadata: DataPointMetadata,
    pub events: Vec<AnalyticsEvent>,
}

// TODO: accept an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
pub struct CalcWasActiveEachPastPeriod {
    pub metadata: DataPointMetadata,
    pub events: Vec<AnalyticsEvent>,
    pub period_thresholds: Vec<DateTime<Utc>>,
}

// TODO: accept an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
pub struct CalcWasActivePastNDays {
    pub metadata: DataPointMetadata,
    pub events: Vec<AnalyticsEvent>,
}
