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

#[derive(Debug, Clone, Copy)]
pub struct DataPointMetadata {
    pub period: Period,
    pub end: DateTime<Utc>,
}

pub trait Calculate: Sized {
    fn metadata(&self) -> DataPointMetadata;

    fn calculate(&self) -> Vec<u32>;
}
#[derive(Debug, Clone)]
pub struct DataPoint {
    metadata: DataPointMetadata,
    values: Vec<u32>,
}

impl DataPoint {
    pub fn new(metadata: DataPointMetadata, values: Vec<u32>) -> DataPoint {
        DataPoint { metadata, values }
    }
}
