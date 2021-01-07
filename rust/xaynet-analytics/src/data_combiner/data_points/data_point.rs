use chrono::{DateTime, Utc};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PeriodUnit {
    Days,
    Weeks,
    Months,
    Any,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Period {
    pub unit: PeriodUnit,
    pub n: u32,
}

impl Period {
    pub fn new(unit: PeriodUnit, n: u32) -> Period {
        Period{ unit, n }
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

pub trait CalculateDataPoints: Sized {
    fn metadata(&self) -> DataPointMetadata;

    fn calculate(&self) -> Vec<u32>;
}
#[derive(Debug, Clone)]
pub struct DataPoints {
    metadata: DataPointMetadata,
    values: Vec<u32>,
}

impl DataPoints {
    pub fn new(metadata: DataPointMetadata, values: Vec<u32>) -> DataPoints {
        DataPoints { metadata, values }
    }
}
