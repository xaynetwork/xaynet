use chrono::{DateTime, Utc};
use isar_core::object::{data_type::DataType, object_builder::ObjectBuilder};
use std::{
    fmt::{Display, Formatter, Result},
    vec::IntoIter,
};

use crate::database::common::{FieldProperty, IsarAdapter};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AnalyticsEventType {
    AppEvent = 0,
    Error = 1,
    ScreenEnter = 2,
    UserAction = 3,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AnalyticsEvent {
    pub name: String,
    pub event_type: AnalyticsEventType,
    pub timestamp: DateTime<Utc>,
    pub screen_route: Option<String>,
}

impl AnalyticsEvent {
    pub fn new<N: Into<String>, R: Into<Option<String>>>(
        name: N,
        event_type: AnalyticsEventType,
        timestamp: DateTime<Utc>,
        screen_route: R,
    ) -> AnalyticsEvent {
        AnalyticsEvent {
            name: name.into(),
            event_type,
            timestamp,
            screen_route: screen_route.into(),
        }
    }

    fn add_screen_route(&self, object_builder: &mut ObjectBuilder) {
        match &self.screen_route {
            Some(screen) => object_builder.write_string(Some(&screen)),
            None => object_builder.write_null(),
        };
    }
}

impl IsarAdapter for AnalyticsEvent {
    fn into_field_properties() -> IntoIter<FieldProperty> {
        // NOTE: properties need to be ordered by type. Properties with the same type need to be ordered alphabetically
        // https://github.com/isar/isar-core/blob/1ea9f27edfd6e3708daa47ac6a17995b628f31a6/src/schema/collection_schema.rs
        vec![
            FieldProperty::new("event_type", DataType::Int),
            FieldProperty::new("name", DataType::String),
            FieldProperty::new("screen_route", DataType::String),
            FieldProperty::new("timestamp", DataType::String),
        ]
        .into_iter()
    }

    fn write_with_object_builder(&self, object_builder: &mut ObjectBuilder) {
        object_builder.write_int(self.event_type as i32);
        object_builder.write_string(Some(&self.name));
        self.add_screen_route(object_builder);
        object_builder.write_string(Some(&self.timestamp.to_rfc3339()));
    }
}

impl Display for AnalyticsEvent {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{:?}_{:?}_{:?}_{:?}",
            self.name,
            self.event_type.to_string(),
            self.timestamp,
            self.screen_route
        )
    }
}

impl Display for AnalyticsEventType {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}
