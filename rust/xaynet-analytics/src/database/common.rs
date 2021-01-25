use anyhow::Error;
use isar_core::{
    index::StringIndexType,
    object::{data_type::DataType, object_builder::ObjectBuilder},
};
use std::vec::IntoIter;

pub trait IsarAdapter: Sized {
    fn into_field_properties() -> IntoIter<FieldProperty>;

    fn write_with_object_builder(&self, object_builder: &mut ObjectBuilder);
}

pub trait Repo<T> {
    fn add(&self, object: &mut T) -> Result<(), Error>;

    fn get_all(&self) -> Result<Vec<T>, Error>;
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
}

impl Repo<MockObject> for MockRepo {
    fn add(&self, _object: &mut MockObject) -> Result<(), Error> {
        unimplemented!()
    }

    fn get_all(&self) -> Result<Vec<MockObject>, Error> {
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
    pub fn new(name: String, data_type: DataType) -> FieldProperty {
        FieldProperty {
            name,
            data_type,
            string_index_type: StringIndexType::Hash,
            is_case_sensitive: true,
            is_unique: true,
        }
    }
}
