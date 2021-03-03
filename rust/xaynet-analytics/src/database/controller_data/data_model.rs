use anyhow::Result;
use chrono::{DateTime, Utc};
use std::convert::{Into, TryFrom};

use crate::database::controller_data::adapter::ControllerDataAdapter;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ControllerData {
    pub time_data_sent: DateTime<Utc>,
}

impl ControllerData {
    pub fn new(time_data_sent: DateTime<Utc>) -> Self {
        Self { time_data_sent }
    }
}

impl TryFrom<ControllerDataAdapter> for ControllerData {
    type Error = anyhow::Error;

    fn try_from(adapter: ControllerDataAdapter) -> Result<Self, Self::Error> {
        Ok(ControllerData::new(
            DateTime::parse_from_rfc3339(&adapter.time_data_sent)?.with_timezone(&Utc),
        ))
    }
}

impl Into<ControllerDataAdapter> for ControllerData {
    fn into(self) -> ControllerDataAdapter {
        ControllerDataAdapter::new(self.time_data_sent.to_rfc3339())
    }
}
