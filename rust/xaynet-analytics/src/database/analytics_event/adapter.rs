use anyhow::{anyhow, Error, Result};
use isar_core::object::{
    data_type::DataType,
    isar_object::{IsarObject, Property},
    object_builder::ObjectBuilder,
};
use std::{convert::TryFrom, vec::IntoIter};

use crate::database::{
    common::{FieldProperty, IsarAdapter, RelationalField, Repo, SchemaGenerator},
    isar::IsarDb,
    screen_route::data_model::ScreenRoute,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AnalyticsEventAdapter {
    pub name: String,
    pub event_type: i32,
    pub timestamp: String,
    pub screen_route_field: Option<String>,
}

impl AnalyticsEventAdapter {
    pub fn new<N: Into<String>>(
        name: N,
        event_type: i32,
        timestamp: String,
        screen_route_field: Option<RelationalField>,
    ) -> Self {
        Self {
            name: name.into(),
            event_type,
            timestamp,
            screen_route_field: screen_route_field.map(|field| field.into()),
        }
    }
}

impl<'event> IsarAdapter<'event> for AnalyticsEventAdapter {
    fn get_oid(&self) -> String {
        format!("{}-{}", self.name, self.timestamp)
    }

    fn into_field_properties() -> IntoIter<FieldProperty> {
        // NOTE: properties need to be ordered by type. Properties with the same type need to be ordered alphabetically
        // https://github.com/isar/isar-core/blob/1ea9f27edfd6e3708daa47ac6a17995b628f31a6/src/schema/collection_schema.rs
        vec![
            FieldProperty::new("event_type", DataType::Int, false),
            FieldProperty::new("name", DataType::String, false),
            FieldProperty::new("oid", DataType::String, true),
            FieldProperty::new("screen_route_field", DataType::String, false),
            FieldProperty::new("timestamp", DataType::String, false),
        ]
        .into_iter()
    }

    fn write_with_object_builder(&self, object_builder: &mut ObjectBuilder) {
        object_builder.write_int(self.event_type);
        object_builder.write_string(Some(&self.name));
        object_builder.write_string(Some(&self.get_oid()));
        object_builder.write_string(self.screen_route_field.as_deref());
        object_builder.write_string(Some(&self.timestamp));
    }

    fn read(
        isar_object: &'event IsarObject,
        isar_properties: &'event [(String, Property)],
    ) -> Result<AnalyticsEventAdapter, Error> {
        let name_property = Self::find_property_by_name("name", isar_properties)?;
        let event_type_property = Self::find_property_by_name("event_type", isar_properties)?;
        let timestamp_property = Self::find_property_by_name("timestamp", isar_properties)?;
        let screen_route_field_property =
            Self::find_property_by_name("screen_route_field", isar_properties)?;

        let name_field = isar_object
            .read_string(name_property)
            .ok_or_else(|| anyhow!("unable to read name"))?;
        let event_type_field = isar_object.read_int(event_type_property);
        let timestamp_field = isar_object
            .read_string(timestamp_property)
            .ok_or_else(|| anyhow!("unable to read timestamp"))?
            .to_string();
        let screen_route_field = isar_object
            .read_string(screen_route_field_property)
            .map(RelationalField::try_from)
            .transpose()?;

        Ok(AnalyticsEventAdapter::new(
            name_field,
            event_type_field,
            timestamp_field,
            screen_route_field,
        ))
    }
}

impl<'event> SchemaGenerator<'event, AnalyticsEventAdapter> for AnalyticsEventAdapter {}

pub struct AnalyticsEventRelationalAdapter {
    pub name: String,
    pub event_type: i32,
    pub timestamp: String,
    pub screen_route: Option<ScreenRoute>,
}

impl AnalyticsEventRelationalAdapter {
    pub fn new(adapter: AnalyticsEventAdapter, db: &IsarDb) -> Result<Self, Error> {
        let screen_route = adapter
            .screen_route_field
            .map(|screen_route_field| {
                let relational_field = RelationalField::try_from(screen_route_field.as_str())?;
                ScreenRoute::get(
                    &relational_field.value,
                    db,
                    &relational_field.collection_name,
                )
            })
            .transpose()?;

        Ok(Self {
            name: adapter.name,
            event_type: adapter.event_type,
            timestamp: adapter.timestamp,
            screen_route,
        })
    }
}
