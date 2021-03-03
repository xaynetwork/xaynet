use anyhow::Result;
use chrono::{DateTime, Utc};
use std::convert::{Into, TryFrom};

use crate::database::{
    common::{CollectionNames, RelationalField},
    screen_route::adapter::ScreenRouteAdapter,
};

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
