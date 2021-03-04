//! In this file `ControllerData` is declared, together with tome conversion methods to/from adapters.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::convert::{Into, TryFrom};

use crate::database::controller_data::adapter::ControllerDataAdapter;

/// Holds some metadata useful for the `AnalyticsController`. For now it only contains `time_data_sent`,
/// which is the time when analytics data was last sent to the coordinator for aggregation.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_data_try_from_adapter() {
        let timestamp_str = "2021-01-01T01:01:00+00:00";
        let timestamp_parsed = DateTime::parse_from_rfc3339(timestamp_str)
            .unwrap()
            .with_timezone(&Utc);
        let controller_data = ControllerData::new(timestamp_parsed);
        let adapter = ControllerDataAdapter::new(timestamp_str);
        assert_eq!(ControllerData::try_from(adapter).unwrap(), controller_data);
    }

    #[test]
    fn test_adapter_into_controller_data() {
        let timestamp_str = "2021-01-01T01:01:00+00:00";
        let timestamp_parsed = DateTime::parse_from_rfc3339(timestamp_str)
            .unwrap()
            .with_timezone(&Utc);
        let controller_data = ControllerData::new(timestamp_parsed);
        let actual_adapter: ControllerDataAdapter = controller_data.into();
        let expected_adapter = ControllerDataAdapter::new(timestamp_str);
        assert_eq!(actual_adapter, expected_adapter);
    }
}
