use anyhow::{anyhow, Error, Result};
use isar_core::object::{
    data_type::DataType,
    isar_object::{IsarObject, Property},
    object_builder::ObjectBuilder,
};
use std::vec::IntoIter;

use crate::database::common::{FieldProperty, IsarAdapter, SchemaGenerator};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ScreenRouteAdapter {
    pub name: String,
    pub created_at: String,
}

impl ScreenRouteAdapter {
    pub fn new(name: String, created_at: String) -> Self {
        Self { name, created_at }
    }
}

impl<'screen> IsarAdapter<'screen> for ScreenRouteAdapter {
    fn into_field_properties() -> IntoIter<FieldProperty> {
        vec![
            FieldProperty::new("created_at".to_string(), DataType::String),
            FieldProperty::new("name".to_string(), DataType::String),
        ]
        .into_iter()
    }

    fn write_with_object_builder(&self, object_builder: &mut ObjectBuilder) {
        object_builder.write_string(Some(&self.created_at));
        object_builder.write_string(Some(&self.name));
    }

    fn read(
        isar_object: &'screen IsarObject,
        isar_properties: &'screen [(String, Property)],
    ) -> Result<ScreenRouteAdapter, Error> {
        let name_property = Self::find_property_by_name("name", isar_properties);
        let created_at_property = Self::find_property_by_name("created_at", isar_properties);

        let name_data = isar_object
            .read_string(name_property?)
            .ok_or_else(|| anyhow!("unable to read name"))?;
        let created_at_data = isar_object
            .read_string(created_at_property?)
            .ok_or_else(|| anyhow!("unable to read created_at"))?;

        Ok(ScreenRouteAdapter::new(
            name_data.to_string(),
            created_at_data.to_string(),
        ))
    }
}

impl<'screen> SchemaGenerator<'screen, ScreenRouteAdapter> for ScreenRouteAdapter {}
