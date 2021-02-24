use anyhow::{anyhow, Error, Result};
use isar_core::{
    index::StringIndexType,
    object::{
        data_type::DataType,
        isar_object::{IsarObject, Property},
        object_builder::ObjectBuilder,
    },
    schema::collection_schema::CollectionSchema,
};
use std::{convert::TryFrom, vec::IntoIter};

use crate::database::isar::IsarDb;

pub trait IsarAdapter<'object>: Sized {
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
    fn add(self, db: &'db IsarDb, collection_name: &str) -> Result<(), Error>;

    fn get_all(db: &'db IsarDb, collection_name: &str) -> Result<Vec<M>, Error>;

    fn get(object_id: &str, db: &'db IsarDb, collection_name: &str) -> Result<M, Error>;
}

pub struct MockRepo {}

pub struct MockObject {}

impl<'object> IsarAdapter<'object> for MockObject {
    fn into_field_properties() -> IntoIter<FieldProperty> {
        unimplemented!()
    }

    fn write_with_object_builder(&self, _object_builder: &mut ObjectBuilder) {
        unimplemented!()
    }

    fn read(
        _isar_object: &'object IsarObject,
        _isar_properties: &'object [(String, Property)],
    ) -> Result<MockObject, Error> {
        unimplemented!()
    }
}

impl<'db> Repo<'db, MockObject> for MockObject {
    fn add(self, _db: &'db IsarDb, _collection_name: &str) -> Result<(), Error> {
        unimplemented!()
    }

    fn get_all(_db: &'db IsarDb, _collection_name: &str) -> Result<Vec<MockObject>, Error> {
        unimplemented!()
    }

    fn get(
        _object_id: &str,
        _db: &'db IsarDb,
        _collection_name: &str,
    ) -> Result<MockObject, Error> {
        unimplemented!()
    }
}

pub struct FieldProperty {
    pub name: String,
    pub data_type: DataType,
    pub string_index_type: StringIndexType,
    pub is_case_sensitive: bool,
    pub is_unique: bool,
}

impl FieldProperty {
    pub fn new(name: String, data_type: DataType) -> Self {
        Self {
            name,
            data_type,
            string_index_type: StringIndexType::Hash,
            is_case_sensitive: true,
            is_unique: true,
        }
    }
}

pub trait SchemaGenerator<'object, A>
where
    A: IsarAdapter<'object>,
{
    fn get_schema(name: &str) -> Result<CollectionSchema, Error> {
        A::into_field_properties().try_fold(
            CollectionSchema::new(name, &format!("{}_oid", name), DataType::String),
            |mut schema, prop| {
                schema
                    .add_property(&prop.name, prop.data_type)
                    .map_err(|_| {
                        anyhow!(
                            "failed to add property {} to collection {}",
                            prop.name,
                            name
                        )
                    })?;
                schema
                    .add_index(
                        &[(
                            &prop.name,
                            Some(prop.string_index_type),
                            prop.is_case_sensitive,
                        )],
                        prop.is_unique,
                    )
                    .map_err(|_| {
                        anyhow!(
                            "failed to add index for {} to collection {}",
                            prop.name,
                            name
                        )
                    })?;
                Ok(schema)
            },
        )
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
        fn stringify(input: Option<&&str>) -> Result<String, Error> {
            Ok(input
                .ok_or_else(|| anyhow!("could not unwrap input {:?}", input))?
                .to_string())
        }

        let data_split: Vec<&str> = data.split('=').collect();
        if !data_split.len() == 2 {
            return Err(anyhow!(
                "data {:?} is not a str made of two elements separated by '='",
                data
            ));
        }

        Ok(Self {
            value: stringify(data_split.first())?,
            collection_name: stringify(data_split.last())?,
        })
    }
}

impl Into<String> for RelationalField {
    fn into(self) -> String {
        format!("{:?}={:?}", self.value, self.collection_name)
    }
}

pub struct CollectionNames;

impl CollectionNames {
    pub const ANALYTICS_EVENTS: &'static str = "analytics_events";
    pub const CONTROLLER_DATA: &'static str = "controller_data";
    pub const SCREEN_ROUTES: &'static str = "screen_routes";
}
