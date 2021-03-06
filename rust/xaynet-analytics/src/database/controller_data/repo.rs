//! Implementations of the methods needed to save and get `ControllerData` to/from Isar.

use anyhow::{anyhow, Error, Result};
use std::convert::{Into, TryFrom};

use crate::database::{
    common::{IsarAdapter, Repo},
    controller_data::{adapter::ControllerDataAdapter, data_model::ControllerData},
    isar::IsarDb,
};

impl<'db> Repo<'db, ControllerData> for ControllerData {
    fn save(self, db: &'db IsarDb, collection_name: &str) -> Result<(), Error> {
        let mut object_builder = db.get_object_builder(collection_name)?;
        let data_adapter: ControllerDataAdapter = self.into();
        data_adapter.write_with_object_builder(&mut object_builder);
        db.put(collection_name, object_builder.finish().as_bytes())
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_all(db: &'db IsarDb, collection_name: &str) -> Result<Vec<Self>, Error> {
        let isar_properties = db.get_collection_properties(collection_name)?;
        db.get_all_isar_objects(collection_name)?
            .into_iter()
            .map(|(_, isar_object)| ControllerDataAdapter::read(&isar_object, isar_properties))
            .map(|data_adapter| ControllerData::try_from(data_adapter?))
            .collect()
    }

    fn get(oid: &str, db: &'db IsarDb, collection_name: &str) -> Result<Self, Error> {
        let isar_properties = db.get_collection_properties(collection_name)?;
        let object_id = db.get_object_id_from_str(collection_name, oid)?;
        let mut transaction = db.get_read_transaction()?;
        let isar_object =
            db.get_isar_object_by_id(&object_id, collection_name, &mut transaction)?;
        if let Some(isar_object) = isar_object {
            let data_adapter = ControllerDataAdapter::read(&isar_object, isar_properties)?;
            ControllerData::try_from(data_adapter)
        } else {
            Err(anyhow!("unable to get {:?} object", object_id))
        }
    }
}
