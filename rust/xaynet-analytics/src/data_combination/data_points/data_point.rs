//! File containing various structs used to define `DataPoints`.

use chrono::{DateTime, Utc};

use crate::database::analytics_event::data_model::AnalyticsEvent;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PeriodUnit {
    Days,
    Weeks,
    Months,
}

/// Period combines information about the unit of this period, and the number of periods.
/// For example a `Period` of three weeks can be represented with `Period::new(unit: PeriodUnit::Weeks, n: 3)`
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Period {
    pub unit: PeriodUnit,
    pub n: u32,
}

impl Period {
    pub fn new(unit: PeriodUnit, n: u32) -> Self {
        Self { unit, n }
    }
}

/// `DataPointMetadata` contains information about `Period` and when the period ends. It is used to
/// define which `AnalyticsEvents` fall inside a `Period` and must therefore be included in the calculation
/// of a specific `DataPoint`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DataPointMetadata {
    pub period: Period,
    pub end: DateTime<Utc>,
}

impl DataPointMetadata {
    pub fn new(period: Period, end: DateTime<Utc>) -> Self {
        Self { period, end }
    }
}

pub trait CalculateDataPoints {
    fn metadata(&self) -> DataPointMetadata;

    fn calculate(&self) -> Vec<u32>;
}

/// `DataPoint` is an enum whose variants represent data points that will need to be aggregated and shown to the user.
/// They are the actual analytics information that is valuable to the user. Each `DataPoint` refers to a specific `Period`.
/// ## Variants:
/// * `ScreenActiveTime`: How much time was spent on a specific screen.
/// * `ScreenEnterCount`: How many times the user entered a specific screen.
/// * `WasActiveEachPastPeriod`: Whether the user was active or not in each specified period (in general, not by screen).
/// * `WasActivePastNDays`: Whether the user was active or not in the past N days (in general, not by screen).
///
/// There are still more variants to be implemented: https://xainag.atlassian.net/browse/XN-1687
#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
// TODO: accept an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
pub struct CalcScreenActiveTime {
    pub metadata: DataPointMetadata,
    pub events: Vec<AnalyticsEvent>,
}

#[derive(Debug, PartialEq, Eq)]
// TODO: accept an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
pub struct CalcScreenEnterCount {
    pub metadata: DataPointMetadata,
    pub events: Vec<AnalyticsEvent>,
}

#[derive(Debug, PartialEq, Eq)]
// TODO: accept an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
pub struct CalcWasActiveEachPastPeriod {
    pub metadata: DataPointMetadata,
    pub events: Vec<AnalyticsEvent>,
    pub period_thresholds: Vec<DateTime<Utc>>,
}

#[derive(Debug, PartialEq, Eq)]
// TODO: accept an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
pub struct CalcWasActivePastNDays {
    pub metadata: DataPointMetadata,
    pub events: Vec<AnalyticsEvent>,
}
