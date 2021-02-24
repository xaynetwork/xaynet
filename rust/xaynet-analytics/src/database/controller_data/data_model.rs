use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use std::convert::{TryFrom, TryInto};

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

impl TryInto<ControllerDataAdapter> for ControllerData {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<ControllerDataAdapter, Error> {
        Ok(ControllerDataAdapter::new(self.time_data_sent.to_rfc3339()))
    }
}
