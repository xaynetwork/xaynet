use anyhow::{anyhow, Error, Result};
use isar_core::{
    collection::IsarCollection,
    instance::IsarInstance,
    object::{
        isar_object::{IsarObject, Property},
        object_builder::ObjectBuilder,
        object_id::ObjectId,
    },
    schema::{collection_schema::CollectionSchema, Schema},
    txn::IsarTxn,
};
use std::sync::Arc;

pub struct IsarDb {
    instance: Arc<IsarInstance>,
}

impl IsarDb {
    const MAX_SIZE: usize = 10000000;

    pub fn new(path: &str, collection_schemas: Vec<CollectionSchema>) -> Result<IsarDb, Error> {
        IsarInstance::open(
            path,
            IsarDb::MAX_SIZE,
            IsarDb::get_schema(collection_schemas)?,
        )
        .map_err(|_| anyhow!("failed to create IsarInstance"))
        .map(|instance| IsarDb { instance })
    }

    pub fn get_all_isar_objects(
        &self,
        collection_name: &str,
    ) -> Result<Vec<(ObjectId, IsarObject)>, Error> {
        self.get_collection(collection_name)?
            .new_query_builder()
            .build()
            .find_all_vec(&mut self.begin_txn(false)?)
            .map_err(|_| {
                anyhow!(
                    "failed to find all bytes from collection {}",
                    collection_name
                )
            })
    }

    pub fn get_transaction(&self) -> Result<IsarTxn, Error> {
        self.begin_txn(false)
    }

    pub fn get_isar_object_by_id<'txn>(
        &self,
        object_id: &ObjectId,
        collection_name: &str,
        transaction: &'txn mut IsarTxn,
    ) -> Result<Option<IsarObject<'txn>>, Error> {
        self.get_collection(collection_name)?
            .get(transaction, object_id)
            .map_err(|err| anyhow!("unable to get {:?} object ({:?})", object_id, err))
    }

    pub fn put(
        &self,
        collection_name: &str,
        object_id: Option<ObjectId>,
        object: &[u8],
    ) -> Result<(), Error> {
        self.get_collection(collection_name)?
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

    pub fn get_object_id_from_str(
        &self,
        collection_name: &str,
        oid: &str,
    ) -> Result<ObjectId, Error> {
        Ok(self.get_collection(collection_name)?.new_string_oid(oid))
    }

    pub fn get_collection_properties(
        &self,
        collection_name: &str,
    ) -> Result<&[(String, Property)], Error> {
        Ok(self.get_collection(collection_name)?.get_properties())
    }

    pub fn dispose(self) -> Result<(), Error> {
        match self.instance.close() {
            Some(_) => Err(anyhow!("could not close the IsarInstance")),
            None => Ok(()),
        }
    }

    fn get_schema(collection_schemas: Vec<CollectionSchema>) -> Result<Schema, Error> {
        collection_schemas
            .iter()
            .try_fold(Schema::new(), |mut schema, collection_schema| {
                schema
                    .add_collection(collection_schema.to_owned())
                    .map_err(|_| anyhow!("failed to add collection schema to instance schema"))?;
                Ok(schema)
            })
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
