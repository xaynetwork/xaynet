//! This file contains traits and structs that are common to other components involved with the database.
//! It could be split up in smaller files, especially if more traits receive a default implementation.
//! See: https://xainag.atlassian.net/browse/XN-1692

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

/// `IsarAdapter` trait needs to be implemented for each data model adapters.
/// This is needed to be able to tell Isar how to write/read objects to/from a collection.
///
/// The implementations of these methods could actually be automated by a macro, since they are always the same.
/// See: https://xainag.atlassian.net/browse/XN-1689
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

/// This trait is implemented directly for each data model to have a high level API for `AnalyticsController` to
/// save/get objects from the db.
///
/// Consider using default implementations here, to reduce boiler plate code in repo.rs files.
/// See: https://xainag.atlassian.net/browse/XN-1688
pub trait Repo<'db, M>
where
    M: Sized,
{
    fn save(self, db: &'db IsarDb, collection_name: &str) -> Result<(), Error>;

    fn get_all(db: &'db IsarDb, collection_name: &str) -> Result<Vec<M>, Error>;

    fn get(object_id: &str, db: &'db IsarDb, collection_name: &str) -> Result<M, Error>;
}

/// `FieldProperty` is a simple struct that holds data used to register properties and indexes for Isar schemas.
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

/// `SchemaGenerator` is needed to register the `PropertySchema` and `IndexSchema` for each `FieldProperty`.
/// `PropertySchema` and `IndexSchema` are imported from Isar, while `FieldProperty` is an internal struct to
/// make it convenient to iterate through each property (see the fold below).
///
/// When `Ok` it returns a `CollectionSchema` that is needed by Isar to manage a collection.
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

/// `RelationalField` is the struct that allows to save data model instances inside other data models.
///
/// ## Arguments
/// * `value` - is a `String` representing an id with which the data model can be identified
/// * `collection_name` - is the name of the collection where the object is saved
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

/// Stores the name of each collection. Whenever you need to make an operation on an `IsarCollection`,
/// these `str`s are needed.
pub struct CollectionNames;

impl CollectionNames {
    pub const ANALYTICS_EVENTS: &'static str = "analytics_events";
    pub const CONTROLLER_DATA: &'static str = "controller_data";
    pub const SCREEN_ROUTES: &'static str = "screen_routes";
}
