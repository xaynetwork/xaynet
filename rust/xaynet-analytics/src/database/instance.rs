use isar_core::{
    collection::IsarCollection,
    error::IsarError,
    instance::IsarInstance,
    object::{object_builder::ObjectBuilder, object_id::ObjectId},
    schema::{collection_schema::CollectionSchema, Schema},
};
use std::vec::IntoIter;

use crate::database::{
    analytics_event::data_model::AnalyticsEvent,
    common::{FieldProperty, IsarAdapter},
};

pub struct DbInstance {
    instance: IsarInstance,
}

impl DbInstance {
    const MAX_SIZE: usize = 10000000;
    const ANALYTICS_EVENT_NAME: &'static str = "analytics_events";

    pub fn new(path: &str) -> Result<Self, IsarError> {
        match IsarInstance::create(path, Self::MAX_SIZE, Self::get_schema()) {
            Ok(instance) => Ok(Self { instance }),
            Err(e) => Err(e),
        }
    }

    pub fn instance(&self) -> &IsarInstance {
        &self.instance
    }

    pub fn get_all_as_bytes(
        &self,
        collection_name: &str,
    ) -> Result<Vec<(&ObjectId, &[u8])>, IsarError> {
        let collection = match self.get_collection(collection_name) {
            Ok(collection) => collection,
            Err(e) => return Err(e),
        };
        let transaction = match self.instance().begin_txn(false) {
            Ok(transaction) => transaction,
            Err(e) => return Err(e),
        };
        let _bytes = match self
            .instance()
            .create_query_builder(collection)
            .build()
            .find_all_vec(&transaction)
        {
            Ok(bytes) => bytes,
            Err(e) => return Err(e),
        };

        // TODO: not sure how to proceed to parse [u8] using the collection schema. didn't find examples in Isar
        unimplemented!()
    }

    pub fn put(&self, collection_name: &str, object: &[u8]) -> Result<(), IsarError> {
        let transaction = match self.instance().begin_txn(false) {
            Ok(transaction) => transaction,
            Err(e) => return Err(e),
        };
        let collection = match self.get_collection(collection_name) {
            Ok(collection) => collection,
            Err(e) => return Err(e),
        };
        match collection.put(&transaction, None, object) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn get_object_builder(&self, collection_name: &str) -> Result<ObjectBuilder, IsarError> {
        match self.get_collection(collection_name) {
            Ok(collection) => Ok(collection.get_object_builder()),
            Err(e) => Err(e),
        }
    }

    fn get_schema() -> Schema {
        let mut schema = Schema::new();
        schema
            .add_collection(get_collection_schema(
                Self::ANALYTICS_EVENT_NAME,
                AnalyticsEvent::into_field_properties(),
            ))
            .ok();
        // here add more collections schemas
        schema
    }

    fn get_collection(&self, collection_name: &str) -> Result<&IsarCollection, IsarError> {
        match self.instance().get_collection_by_name(collection_name) {
            Some(collection) => Ok(collection),
            None => Err(IsarError::IllegalArg {
                message: "wrong collection name".to_string(),
            }),
        }
    }
}

fn get_collection_schema(
    name: &str,
    field_properties: IntoIter<FieldProperty>,
) -> CollectionSchema {
    let mut schema = CollectionSchema::new(&name);
    field_properties.for_each(|prop| {
        schema.add_property(prop.name, prop.data_type).ok();
        schema
            .add_index(&[prop.name], prop.is_unique, prop.has_hash_value)
            .ok();
    });
    schema
}
