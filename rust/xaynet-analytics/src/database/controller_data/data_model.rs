use chrono::{DateTime, Utc};
use isar_core::object::{data_type::DataType, object_builder::ObjectBuilder};
use std::vec::IntoIter;

use crate::database::common::{FieldProperty, IsarAdapter, SchemaGenerator};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ControllerData {
    pub time_data_sent: DateTime<Utc>,
}

impl ControllerData {
    pub fn new(time_data_sent: DateTime<Utc>) -> ControllerData {
        ControllerData { time_data_sent }
    }
}

impl IsarAdapter for ControllerData {
    fn into_field_properties() -> IntoIter<FieldProperty> {
        vec![FieldProperty::new(
            "time_data_sent".to_string(),
            DataType::String,
        )]
        .into_iter()
    }

    fn write_with_object_builder(&self, object_builder: &mut ObjectBuilder) {
        object_builder.write_string(Some(&self.time_data_sent.to_rfc3339()));
    }

    fn read(_bytes: &[u8]) -> ControllerData {
        // TODO: implement when Isar will support it: https://xainag.atlassian.net/browse/XN-1604
        unimplemented!()
    }
}

impl SchemaGenerator<ControllerData> for ControllerData {}
