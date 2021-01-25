use anyhow::{anyhow, Error, Result};
use isar_core::{
    collection::IsarCollection,
    instance::IsarInstance,
    object::{
        data_type::DataType,
        isar_object::IsarObject,
        object_builder::ObjectBuilder,
        object_id::ObjectId,
    },
    schema::{collection_schema::CollectionSchema, Schema},
    txn::IsarTxn,
};
use std::{sync::Arc, vec::IntoIter};

use crate::database::{
    analytics_event::data_model::AnalyticsEvent,
    common::{FieldProperty, IsarAdapter},
};

pub struct IsarDb {
    instance: Arc<IsarInstance>,
}

impl IsarDb {
    const MAX_SIZE: usize = 10000000;
    const ANALYTICS_EVENT_NAME: &'static str = "analytics_events";

    pub fn new(path: &str) -> Result<IsarDb, Error> {
        IsarInstance::open(path, IsarDb::MAX_SIZE, IsarDb::get_schema()?)
            .map_err(|_| anyhow!("failed to create IsarInstance"))
            .map(|instance| IsarDb { instance })
    }

    pub fn get_all_as_bytes(
        &self,
        collection_name: &str,
    ) -> Result<Vec<(&ObjectId, &[u8])>, Error> {
        let _bytes = self
            .get_collection(collection_name)?
            .new_query_builder()
            .build()
            .find_all_vec(&mut self.begin_txn(false)?)
            .map_err(|_| {
                anyhow!(
                    "failed to find all bytes from collection {}",
                    collection_name
                )
            });

        // TODO: not sure how to proceed to parse [u8] using the collection schema. didn't find examples in Isar
        unimplemented!()
    }

    pub fn put(
        &self,
        collection_name: &str,
        object_id: Option<ObjectId>,
        object: &[u8],
    ) -> Result<(), Error> {
        let collection = self.get_collection(collection_name)?;
        collection
            .put(
                &mut self.begin_txn(true)?,
                object_id,
                IsarObject::new(object),
            )
            .map_err(|_| {
                anyhow!(
                    "failed to add object {:?} to collection: {}",
                    object,
                    collection_name
                )
            })
            .map(|_| ())
    }

    pub fn get_object_builder(&self, collection_name: &str) -> Result<ObjectBuilder, Error> {
        Ok(self
            .get_collection(collection_name)?
            .new_object_builder(None))
    }

    fn get_schema() -> Result<Schema, Error> {
        let mut schema = Schema::new();
        schema
            .add_collection(get_collection_schema(
                Self::ANALYTICS_EVENT_NAME,
                &mut AnalyticsEvent::into_field_properties(),
            )?)
            .map_err(|_| {
                anyhow!(
                    "failed to add collection {} to schema",
                    Self::ANALYTICS_EVENT_NAME
                )
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
            .map_err(|_| anyhow!("failed to begin transaction"))
    }
}

fn get_collection_schema(
    name: &str,
    field_properties: &mut IntoIter<FieldProperty>,
) -> Result<CollectionSchema, Error> {
    field_properties.try_fold(
        CollectionSchema::new(&name, &format!("{}_oid", &name), DataType::String),
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
