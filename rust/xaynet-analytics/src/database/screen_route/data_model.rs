//! In this file `ScreenRoute` is declared, together with tome conversion methods to/from adapters.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::convert::{Into, TryFrom};

use crate::database::{
    common::{CollectionNames, RelationalField},
    screen_route::adapter::ScreenRouteAdapter,
};

/// A `ScreenRoute` is the internal representation of a screen in the app installing the library.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ScreenRoute {
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl ScreenRoute {
    pub fn new<N: Into<String>>(name: N, created_at: DateTime<Utc>) -> Self {
        Self {
            name: name.into(),
            created_at,
        }
    }
}

impl TryFrom<ScreenRouteAdapter> for ScreenRoute {
    type Error = anyhow::Error;

    fn try_from(adapter: ScreenRouteAdapter) -> Result<Self, Self::Error> {
        Ok(ScreenRoute::new(
            adapter.name,
            DateTime::parse_from_rfc3339(&adapter.created_at)?.with_timezone(&Utc),
        ))
    }
}

impl Into<ScreenRouteAdapter> for ScreenRoute {
    fn into(self) -> ScreenRouteAdapter {
        ScreenRouteAdapter::new(self.name, self.created_at.to_rfc3339())
    }
}

impl From<ScreenRoute> for RelationalField {
    fn from(screen_route: ScreenRoute) -> Self {
        Self {
            value: screen_route.name,
            collection_name: CollectionNames::SCREEN_ROUTES.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_route_try_from_adapter() {
        let timestamp_str = "2021-01-01T01:01:00+00:00";
        let timestamp_parsed = DateTime::parse_from_rfc3339(timestamp_str)
            .unwrap()
            .with_timezone(&Utc);
        let screen_route = ScreenRoute::new("route", timestamp_parsed);
        let adapter = ScreenRouteAdapter::new("route", timestamp_str);
        assert_eq!(ScreenRoute::try_from(adapter).unwrap(), screen_route);
    }

    #[test]
    fn test_adapter_into_screen_route() {
        let timestamp_str = "2021-01-01T01:01:00+00:00";
        let timestamp_parsed = DateTime::parse_from_rfc3339(timestamp_str)
            .unwrap()
            .with_timezone(&Utc);
        let screen_route = ScreenRoute::new("route", timestamp_parsed);
        let actual_adapter: ScreenRouteAdapter = screen_route.into();
        let expected_adapter = ScreenRouteAdapter::new("route", timestamp_str);
        assert_eq!(actual_adapter, expected_adapter);
    }

    #[test]
    fn test_screen_route_from_relational_field() {
        let timestamp_str = "2021-01-01T01:01:00+00:00";
        let timestamp_parsed = DateTime::parse_from_rfc3339(timestamp_str)
            .unwrap()
            .with_timezone(&Utc);
        let screen_route = ScreenRoute::new("route", timestamp_parsed);
        let relational_field = RelationalField {
            value: "route".to_string(),
            collection_name: CollectionNames::SCREEN_ROUTES.to_string(),
        };
        assert_eq!(RelationalField::from(screen_route), relational_field);
    }
}
