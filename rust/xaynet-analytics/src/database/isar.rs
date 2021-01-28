use anyhow::{anyhow, Error, Result};
use isar_core::{
    collection::IsarCollection,
    instance::IsarInstance,
    object::{isar_object::IsarObject, object_builder::ObjectBuilder, object_id::ObjectId},
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

    pub fn dispose(self) -> Result<(), Error> {
        match self.instance.close() {
            Some(_) => Err(anyhow!("err")),
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
