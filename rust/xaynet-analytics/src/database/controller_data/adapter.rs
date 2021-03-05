//! This file contains struct and impl for `ControllerDataAdapter` the implementation of `IsarAdapter`
//! for `ControllerDataAdapter`.

use anyhow::{anyhow, Error, Result};
use isar_core::object::{
    data_type::DataType,
    isar_object::{IsarObject, Property},
    object_builder::ObjectBuilder,
};
use std::vec::IntoIter;

use crate::database::common::{FieldProperty, IsarAdapter, SchemaGenerator};

/// Allows to convert an IsarObject from the db to a `ControllerData`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ControllerDataAdapter {
    pub time_data_sent: String,
}

impl ControllerDataAdapter {
    pub fn new<T: Into<String>>(time_data_sent: T) -> Self {
        Self {
            time_data_sent: time_data_sent.into(),
        }
    }
}

impl<'ctrl> IsarAdapter<'ctrl> for ControllerDataAdapter {
    fn get_oid(&self) -> String {
        self.time_data_sent.clone()
    }

    fn into_field_properties() -> IntoIter<FieldProperty> {
        vec![
            FieldProperty::new("oid", DataType::String, true),
            FieldProperty::new("time_data_sent", DataType::String, false),
        ]
        .into_iter()
    }

    fn write_with_object_builder(&self, object_builder: &mut ObjectBuilder) {
        object_builder.write_string(Some(&self.get_oid()));
        object_builder.write_string(Some(&self.time_data_sent));
    }

    fn read(
        isar_object: &'ctrl IsarObject,
        isar_properties: &'ctrl [(String, Property)],
    ) -> Result<ControllerDataAdapter, Error> {
        let time_data_sent_property =
            Self::find_property_by_name("time_data_sent", isar_properties)?;

        let time_data_sent_data = isar_object
            .read_string(time_data_sent_property)
            .ok_or_else(|| anyhow!("unable to read time_data_sent"))?;

        Ok(ControllerDataAdapter::new(time_data_sent_data))
    }
}

impl<'ctrl> SchemaGenerator<'ctrl, ControllerDataAdapter> for ControllerDataAdapter {}
