use anyhow::{anyhow, Error, Result};
use isar_core::{
    index::IndexType,
    object::{
        data_type::DataType,
        isar_object::{IsarObject, Property},
        object_builder::ObjectBuilder,
    },
    schema::collection_schema::{
        CollectionSchema,
        IndexPropertySchema,
        IndexSchema,
        PropertySchema,
    },
};
use std::{convert::TryFrom, vec::IntoIter};

use crate::database::isar::IsarDb;

pub trait IsarAdapter<'object>: Sized {
    fn get_oid(&self) -> String;

    fn into_field_properties() -> IntoIter<FieldProperty>;

    fn write_with_object_builder(&self, object_builder: &mut ObjectBuilder);

    fn read(
        isar_object: &'object IsarObject,
        isar_properties: &'object [(String, Property)],
    ) -> Result<Self, Error>;

    fn find_property_by_name(
        name: &str,
        isar_properties: &[(String, Property)],
    ) -> Result<Property, Error> {
        isar_properties
            .iter()
            .find(|(isar_property_name, _)| isar_property_name == name)
            .map(|(_, property)| *property)
            .ok_or_else(|| anyhow!("failed to retrieve property {:?}", name))
    }
}

pub trait Repo<'db, M>
where
    M: Sized,
{
    fn save(self, db: &'db IsarDb, collection_name: &str) -> Result<(), Error>;

    fn get_all(db: &'db IsarDb, collection_name: &str) -> Result<Vec<M>, Error>;

    fn get(object_id: &str, db: &'db IsarDb, collection_name: &str) -> Result<M, Error>;
}

pub struct FieldProperty {
    pub name: String,
    pub data_type: DataType,
    pub is_oid: bool,
    pub index_type: IndexType,
    pub is_case_sensitive: bool,
    pub is_unique: bool,
}

impl FieldProperty {
    pub fn new<N: Into<String>>(name: N, data_type: DataType, is_oid: bool) -> Self {
        Self {
            name: name.into(),
            data_type,
            is_oid,
            index_type: IndexType::Value,
            is_case_sensitive: data_type == DataType::String,
            is_unique: true,
        }
    }
}

pub trait SchemaGenerator<'object, A>
where
    A: IsarAdapter<'object>,
{
    fn get_schema(name: &str) -> Result<CollectionSchema, Error> {
        let (properties, indexes) = A::into_field_properties().fold(
            (Vec::new(), Vec::new()),
            |(mut properties, mut indexes), prop| {
                let property_schema = PropertySchema::new(&prop.name, prop.data_type, prop.is_oid);
                let is_index_case_sensitive =
                    Some(true).filter(|_| prop.data_type == DataType::String);
                let index_property_schema = vec![IndexPropertySchema::new(
                    &prop.name,
                    prop.index_type,
                    is_index_case_sensitive,
                )];
                let index_schema = IndexSchema::new(index_property_schema, prop.is_unique);
                properties.push(property_schema);
                indexes.push(index_schema);
                (properties, indexes)
            },
        );
        Ok(CollectionSchema::new(name, properties, indexes))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RelationalField {
    pub value: String,
    pub collection_name: String,
}

// NOTE: when split_once gets to stable, it would be a much better solution for this
// https://doc.rust-lang.org/std/string/struct.String.html#method.split_once
impl TryFrom<&str> for RelationalField {
    type Error = anyhow::Error;

    fn try_from(data: &str) -> Result<Self, Error> {
        let data_split: Vec<&str> = data.split('=').collect();
        if data_split.len() != 2 {
            return Err(anyhow!(
                "data {:?} is not a str made of two elements separated by '='",
                data
            ));
        }

        Ok(Self {
            value: data_split[0].to_string(),
            collection_name: data_split[1].to_string(),
        })
    }
}

impl Into<String> for RelationalField {
    fn into(self) -> String {
        [self.value, self.collection_name].join("=")
    }
}

pub struct CollectionNames;

impl CollectionNames {
    pub const ANALYTICS_EVENTS: &'static str = "analytics_events";
    pub const CONTROLLER_DATA: &'static str = "controller_data";
    pub const SCREEN_ROUTES: &'static str = "screen_routes";
}
