//! In this file `AnalyticsEvent` and `AnalyticsEventType` are declared, together with some conversion methods to/from adapters.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::convert::{From, Into, TryFrom, TryInto};

use crate::database::{
    analytics_event::adapter::{AnalyticsEventAdapter, AnalyticsEventRelationalAdapter},
    common::RelationalField,
    screen_route::data_model::ScreenRoute,
};

/// The type of `AnalyticsEvent` recorded on the framework side.
/// ## Variants:
/// * `AppEvent`: It refes to Flutter's `AppLifeCyclesEvents` (of the equivalent in other frameworks):
///   https://flutter.dev/docs/get-started/flutter-for/android-devs#how-do-i-listen-to-android-activity-lifecycle-events
/// * `AppError`: A known error logged by the developers
/// * `ScreenEnter`: Registers when the user enters a specific screen
/// * `UserAction`: A custom event logged by the developer (eg: clicked on a specific button)
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

/// The core data model of the library. It represents an event recorded on the mobile framework side.
/// It can be logged manually by the developers, or automatically detected by Flutter/the mobile framework side.
/// ## Fields:
/// * `name`: The name of the event.
/// * `event_type`: The type of event.
/// * `timestamp`: When the event was created.
/// * `screen_route`: Optional field representing the screen on which the event was recorded.
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

impl Into<AnalyticsEventAdapter> for AnalyticsEvent {
    fn into(self) -> AnalyticsEventAdapter {
        AnalyticsEventAdapter::new(
            self.name,
            self.event_type as i32,
            self.timestamp.to_rfc3339(),
            self.screen_route.map(RelationalField::from),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::common::CollectionNames;

    #[test]
    fn test_analytics_event_type_try_from_valid_i32() {
        assert_eq!(
            AnalyticsEventType::try_from(0).unwrap(),
            AnalyticsEventType::AppEvent
        );
        assert_eq!(
            AnalyticsEventType::try_from(1).unwrap(),
            AnalyticsEventType::AppError
        );
        assert_eq!(
            AnalyticsEventType::try_from(2).unwrap(),
            AnalyticsEventType::ScreenEnter
        );
        assert_eq!(
            AnalyticsEventType::try_from(3).unwrap(),
            AnalyticsEventType::UserAction
        );
    }

    #[test]
    fn test_analytics_event_type_invalid_i32() {
        assert!(AnalyticsEventType::try_from(42).is_err());
    }

    #[test]
    fn test_analytics_event_try_from_relational_adapter_without_screen_route() {
        let timestamp = "2021-01-01T01:01:00+00:00";
        let relational_adapter = AnalyticsEventRelationalAdapter {
            name: "test".to_string(),
            event_type: 0,
            timestamp: timestamp.to_string(),
            screen_route: None,
        };
        let analytics_event = AnalyticsEvent::new(
            "test",
            AnalyticsEventType::AppEvent,
            DateTime::parse_from_rfc3339(timestamp)
                .unwrap()
                .with_timezone(&Utc),
            None,
        );
        assert_eq!(
            AnalyticsEvent::try_from(relational_adapter).unwrap(),
            analytics_event
        );
    }

    #[test]
    fn test_analytics_event_try_from_relational_adapter_with_screen_route() {
        let timestamp_str = "2021-01-01T01:01:00+00:00";
        let timestamp_parsed = DateTime::parse_from_rfc3339(timestamp_str)
            .unwrap()
            .with_timezone(&Utc);
        let screen_route = ScreenRoute::new("route", timestamp_parsed);
        let relational_adapter = AnalyticsEventRelationalAdapter {
            name: "test".to_string(),
            event_type: 2,
            timestamp: timestamp_str.to_string(),
            screen_route: Some(screen_route.clone()),
        };
        let analytics_event = AnalyticsEvent::new(
            "test",
            AnalyticsEventType::ScreenEnter,
            timestamp_parsed,
            Some(screen_route),
        );
        assert_eq!(
            AnalyticsEvent::try_from(relational_adapter).unwrap(),
            analytics_event
        );
    }

    #[test]
    fn test_analytics_event_try_into_adapter_without_screen_route() {
        let timestamp_str = "2021-01-01T01:01:00+00:00";
        let timestamp_parsed = DateTime::parse_from_rfc3339(timestamp_str)
            .unwrap()
            .with_timezone(&Utc);
        let analytics_event =
            AnalyticsEvent::new("test", AnalyticsEventType::AppError, timestamp_parsed, None);

        let actual_analytics_event_adapter: AnalyticsEventAdapter =
            analytics_event.try_into().unwrap();
        let expected_analytics_event_adapter =
            AnalyticsEventAdapter::new("test", 1, timestamp_str.to_string(), None);
        assert_eq!(
            actual_analytics_event_adapter,
            expected_analytics_event_adapter
        );
    }

    #[test]
    fn test_analytics_event_try_into_adapter_with_screen_route() {
        let timestamp_str = "2021-01-01T01:01:00+00:00";
        let timestamp_parsed = DateTime::parse_from_rfc3339(timestamp_str)
            .unwrap()
            .with_timezone(&Utc);
        let screen_route = ScreenRoute::new("route", timestamp_parsed);
        let relationa_field = RelationalField {
            value: "route".to_string(),
            collection_name: CollectionNames::SCREEN_ROUTES.to_string(),
        };
        let analytics_event = AnalyticsEvent::new(
            "test",
            AnalyticsEventType::UserAction,
            timestamp_parsed,
            Some(screen_route),
        );

        let actual_analytics_event_adapter: AnalyticsEventAdapter =
            analytics_event.try_into().unwrap();
        let expected_analytics_event_adapter =
            AnalyticsEventAdapter::new("test", 3, timestamp_str.to_string(), Some(relationa_field));
        assert_eq!(
            actual_analytics_event_adapter,
            expected_analytics_event_adapter
        );
    }
}
