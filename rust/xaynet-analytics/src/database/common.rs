use anyhow::{anyhow, Error, Result};
use isar_core::{
    index::StringIndexType,
    object::{data_type::DataType, object_builder::ObjectBuilder},
    schema::collection_schema::CollectionSchema,
};
use std::vec::IntoIter;

use crate::database::isar::IsarDb;

pub trait IsarAdapter: Sized {
    fn into_field_properties() -> IntoIter<FieldProperty>;

    fn write_with_object_builder(&self, object_builder: &mut ObjectBuilder);

    fn read(bytes: &[u8]) -> Self;
}

pub trait Repo<T>
where
    T: IsarAdapter,
{
    fn add(&self, object: &T, db: &IsarDb) -> Result<(), Error>;

    fn get_all(&self, db: &IsarDb) -> Result<Vec<T>, Error>;
}

pub struct MockRepo {}

pub struct MockObject {}

impl IsarAdapter for MockObject {
    fn into_field_properties() -> IntoIter<FieldProperty> {
        unimplemented!()
    }

    fn write_with_object_builder(&self, _object_builder: &mut ObjectBuilder) {
        unimplemented!()
    }

    fn read(_bytes: &[u8]) -> MockObject {
        unimplemented!()
    }
}

impl Repo<MockObject> for MockRepo {
    fn add(&self, _object: &MockObject, _db: &IsarDb) -> Result<(), Error> {
        unimplemented!()
    }

    fn get_all(&self, _db: &IsarDb) -> Result<Vec<MockObject>, Error> {
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

pub trait SchemaGenerator<T>
where
    T: IsarAdapter,
{
    fn get_schema(name: &str) -> Result<CollectionSchema, Error> {
        T::into_field_properties().try_fold(
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
