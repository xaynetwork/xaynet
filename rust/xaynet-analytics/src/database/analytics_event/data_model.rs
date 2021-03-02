use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::convert::{From, Into, TryFrom, TryInto};

use crate::database::{
    analytics_event::adapter::{AnalyticsEventAdapter, AnalyticsEventRelationalAdapter},
    common::RelationalField,
    screen_route::data_model::ScreenRoute,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AnalyticsEventType {
    AppEvent = 0,
    AppError = 1,
    ScreenEnter = 2,
    UserAction = 3,
}

impl TryFrom<i32> for AnalyticsEventType {
    type Error = anyhow::Error;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            x if x == AnalyticsEventType::AppEvent as i32 => Ok(AnalyticsEventType::AppEvent),
            x if x == AnalyticsEventType::AppError as i32 => Ok(AnalyticsEventType::AppError),
            x if x == AnalyticsEventType::ScreenEnter as i32 => Ok(AnalyticsEventType::ScreenEnter),
            x if x == AnalyticsEventType::UserAction as i32 => Ok(AnalyticsEventType::UserAction),
            _ => Err(anyhow!(
                "i32 value {:?} is not mapped to an AnalyticsEventType variant",
                v
            )),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AnalyticsEvent {
    pub name: String,
    pub event_type: AnalyticsEventType,
    pub timestamp: DateTime<Utc>,
    pub screen_route: Option<ScreenRoute>,
}

impl AnalyticsEvent {
    pub fn new<N: Into<String>>(
        name: N,
        event_type: AnalyticsEventType,
        timestamp: DateTime<Utc>,
        screen_route: Option<ScreenRoute>,
    ) -> Self {
        Self {
            name: name.into(),
            event_type,
            timestamp,
            screen_route,
        }
    }
}

impl TryFrom<AnalyticsEventRelationalAdapter> for AnalyticsEvent {
    type Error = anyhow::Error;

    fn try_from(adapter: AnalyticsEventRelationalAdapter) -> Result<Self, Self::Error> {
        let event = AnalyticsEvent::new(
            adapter.name,
            adapter
                .event_type
                .try_into()
                .map_err(|_| anyhow!("unable to convert event_type into enum"))?,
            DateTime::parse_from_rfc3339(&adapter.timestamp)?.with_timezone(&Utc),
            adapter.screen_route,
        );
        Ok(event)
    }
}

impl TryInto<AnalyticsEventAdapter> for AnalyticsEvent {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<AnalyticsEventAdapter, Self::Error> {
        Ok(AnalyticsEventAdapter::new(
            self.name,
            self.event_type as i32,
            self.timestamp.to_rfc3339(),
            self.screen_route.map(RelationalField::from),
        ))
    }
}
