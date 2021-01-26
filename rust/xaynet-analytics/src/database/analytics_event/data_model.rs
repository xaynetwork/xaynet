use chrono::{DateTime, Utc};
use isar_core::object::{data_type::DataType, object_builder::ObjectBuilder};
use std::vec::IntoIter;

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
}

impl IsarAdapter for AnalyticsEvent {
    fn into_field_properties() -> IntoIter<FieldProperty> {
        vec![
            FieldProperty::new("name", DataType::String, None, None),
            FieldProperty::new("event_type", DataType::Int, None, None),
            FieldProperty::new("timestamp", DataType::String, None, None),
            FieldProperty::new("screen_route", DataType::String, None, None),
            /* TODO: when ScreenRoute will be a struct, the above IndexProperty will need to reference the id of the ScreenRoute object, like:
             * IndexProperty::new("screen_route_id", DataType::Int, None, None), */
        ]
        .into_iter()
    }

    fn write_with_object_builder(&self, object_builder: &mut ObjectBuilder) {
        object_builder.write_string(Some(&self.name));
        object_builder.write_int(self.event_type as i32);
        object_builder.write_string(Some(&self.timestamp.to_rfc3339()));
        match &self.screen_route {
            Some(screen) => object_builder.write_string(Some(&screen)),
            None => object_builder.write_null(),
        };
    }
}
