use chrono::{DateTime, Utc};
use isar_core::object::{data_type::DataType, object_builder::ObjectBuilder};
use std::vec::IntoIter;

use crate::database::common::{FieldProperty, IsarAdapter, SchemaGenerator};

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

impl IsarAdapter for ScreenRoute {
    fn into_field_properties() -> IntoIter<FieldProperty> {
        vec![
            FieldProperty::new("created_at".to_string(), DataType::String),
            FieldProperty::new("name".to_string(), DataType::String),
        ]
        .into_iter()
    }

    fn write_with_object_builder(&self, object_builder: &mut ObjectBuilder) {
        object_builder.write_string(Some(&self.created_at.to_rfc3339()));
        object_builder.write_string(Some(&self.name));
    }

    fn read(_bytes: &[u8]) -> ScreenRoute {
        // TODO: implement when Isar will support it: https://xainag.atlassian.net/browse/XN-1604
        todo!()
    }
}

impl SchemaGenerator<ScreenRoute> for ScreenRoute {}
