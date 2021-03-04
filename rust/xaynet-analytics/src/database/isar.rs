//! `IsarDb` is an internal abstraction on top of Isar that wraps `IsarInstance`, the main singleton from Isar.

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

/// `IsarDb` is the internal singleton wrapping the `IsarInstance`, which is the singleton coming from Isar.
/// `IsarDb` exposes public methods for the `AnalyticsController` to save/get models via the `Repo` impls and the adapters.
pub struct IsarDb {
    instance: Arc<IsarInstance>,
}

impl IsarDb {
    const MAX_SIZE: usize = 10000000;

    /// `IsarInstance` is the singleton from Isar that coordinates the whole database.
    ///
    /// `Vec<CollectionSchema>` is required by Isar to register each data model `IsarCollection`.
    /// A `IsarCollection` organises data for a single data model (eg: `AnalyticsEvents`).
    pub fn new(path: &str, collection_schemas: Vec<CollectionSchema>) -> Result<IsarDb, Error> {
        IsarInstance::open(
            path,
            IsarDb::MAX_SIZE,
            IsarDb::get_schema(collection_schemas)?,
        )
        .map_err(|error| anyhow!("failed to create IsarInstance: {:?}", error))
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
            .map_err(|error| {
                anyhow!(
                    "failed to find all objects from collection {}: {:?}",
                    collection_name,
                    error,
                )
            })
    }

    /// Transactions are needed to write and read from Isar.
    /// This method is public because it's called inside `Repo::read()`, before passing it to `get_isar_object_by_id()`,
    /// so that the transaction is in scope when called, and the lifetimes are valid.
    pub fn get_read_transaction(&self) -> Result<IsarTxn, Error> {
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
            .map_err(|error| anyhow!("unable to get {:?} object ({:?})", object_id, error))
    }

    pub fn put(&self, collection_name: &str, object: &[u8]) -> Result<(), Error> {
        let mut transaction = self.begin_txn(true)?;
        self.get_collection(collection_name)?
            .put(&mut transaction, IsarObject::new(object))
            .and_then(|_| transaction.commit())
            .map_err(|error| {
                anyhow!(
                    "failed to add object {:?} to collection: {} | {:?}",
                    object,
                    collection_name,
                    error,
                )
            })
    }

    pub fn get_object_builder(&self, collection_name: &str) -> Result<ObjectBuilder, Error> {
        Ok(self
            .get_collection(collection_name)?
            .new_object_builder(None))
    }

    /// When `Ok`, this method returns a valid `ObjectId` that can be used to retrieve a single object from a collection.
    pub fn get_object_id_from_str(
        &self,
        collection_name: &str,
        oid: &str,
    ) -> Result<ObjectId, Error> {
        self.get_collection(collection_name)?
            .new_string_oid(oid)
            .map_err(|error| anyhow!("could not get the object id from {:?}: {:?}", oid, error))
    }

    /// Returns the properties from a collection that were registered via the `CollectionSchema`, and are needed to
    /// read/write objects to/from the collection.
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

    /// The `Schema` is needed to open the `IsarInstance` and is automatically produced by Isar
    /// based on the `Vec<CollectionSchema>` provided when calling `IsarDb::new()`.
    fn get_schema(collection_schemas: Vec<CollectionSchema>) -> Result<Schema, Error> {
        Schema::new(collection_schemas).map_err(|error| {
            anyhow!(
                "failed to add collection schemas to instance schema: {:?}",
                error
            )
        })
    }

    fn get_collection(&self, collection_name: &str) -> Result<&IsarCollection, Error> {
        self.instance
            .get_collection_by_name(collection_name)
            .ok_or_else(|| anyhow!("wrong collection name: {}", collection_name))
    }

    /// Transactions are needed to read/write objects from Isar. Write transactions should stay private.
    fn begin_txn(&self, is_write: bool) -> Result<IsarTxn, Error> {
        self.instance
            .begin_txn(is_write)
            .map_err(|error| anyhow!("failed to begin transaction: {:?}", error))
    }
}
