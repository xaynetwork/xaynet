use anyhow::{anyhow, Error, Result};

use isar_core::{
    collection::IsarCollection,
    instance::IsarInstance,
    object::{object_builder::ObjectBuilder, object_id::ObjectId},
    schema::{collection_schema::CollectionSchema, Schema},
    txn::IsarTxn,
};
use std::vec::IntoIter;

use crate::database::{
    analytics_event::data_model::AnalyticsEvent,
    common::{FieldProperty, IsarAdapter},
};

pub struct IsarDb {
    instance: IsarInstance,
}

impl IsarDb {
    const MAX_SIZE: usize = 10000000;
    const ANALYTICS_EVENT_NAME: &'static str = "analytics_events";

    pub fn new(path: &str) -> Result<Self, Error> {
        IsarInstance::create(path, Self::MAX_SIZE, Self::get_schema()?)
            .or_else(|_| Err(anyhow!("failed to create IsarInstance")))
            .map(|instance| Self { instance })
    }

    pub fn get_all_as_bytes(
        &self,
        collection_name: &str,
    ) -> Result<Vec<(&ObjectId, &[u8])>, Error> {
        let _bytes = self
            .instance
            .create_query_builder(self.get_collection(collection_name)?)
            .build()
            .find_all_vec(&self.begin_txn(false)?)
            .or_else(|_| {
                Err(anyhow!(
                    "failed to find all bytes from collection {}",
                    collection_name
                ))
            });

        // TODO: not sure how to proceed to parse [u8] using the collection schema. didn't find examples in Isar
        unimplemented!()
    }

    pub fn put(&self, collection_name: &str, object: &[u8]) -> Result<(), Error> {
        self.get_collection(collection_name)?
            .put(&self.begin_txn(false)?, None, object)
            .or_else(|_| {
                Err(anyhow!(
                    "failed to add object {:?} to collection: {}",
                    object,
                    collection_name
                ))
            })
            .map(|_| ())
    }

    pub fn get_object_builder(&self, collection_name: &str) -> Result<ObjectBuilder, Error> {
        Ok(self.get_collection(collection_name)?.get_object_builder())
    }

    fn get_schema() -> Result<Schema, Error> {
        let mut schema = Schema::new();
        schema
            .add_collection(get_collection_schema(
                Self::ANALYTICS_EVENT_NAME,
                AnalyticsEvent::into_field_properties(),
            )?)
            .or_else(|_| {
                Err(anyhow!(
                    "failed to add collection {} to schema",
                    Self::ANALYTICS_EVENT_NAME
                ))
            })
            .map(|_| schema)
    }

    fn get_collection(&self, collection_name: &str) -> Result<&IsarCollection, Error> {
        self.instance
            .get_collection_by_name(collection_name)
            .ok_or_else(|| anyhow!("wrong collection name: {}", collection_name))
    }

    fn begin_txn(&self, write: bool) -> Result<IsarTxn, Error> {
        self.instance
            .begin_txn(write)
            .or_else(|_| Err(anyhow!("failed to begin transaction")))
    }
}

fn get_collection_schema(
    name: &str,
    field_properties: IntoIter<FieldProperty>,
) -> Result<CollectionSchema, Error> {
    let mut schema = CollectionSchema::new(&name);
    field_properties.for_each(|prop| {
        schema
            .add_property(prop.name, prop.data_type)
            .map_err(|_| {
                anyhow!(
                    "failed to add property {} to collection {}",
                    prop.name,
                    name
                )
            })
            .ok();
        schema
            .add_index(&[prop.name], prop.is_unique, prop.has_hash_value)
            .map_err(|_| {
                anyhow!(
                    "failed to add index for {} to collection {}",
                    prop.name,
                    name
                )
            })
            .ok();
    });
    Ok(schema)
}
