use anyhow::{Error, Result};

use crate::database::{
    common::{IsarAdapter, Repo},
    controller_data::data_model::ControllerData,
    isar::IsarDb,
};

pub struct ControllerDataRepo<'db> {
    collection_name: &'db str,
}

impl<'db> ControllerDataRepo<'db> {
    pub fn new(collection_name: &'db str) -> Self {
        Self { collection_name }
    }
}

impl<'db> Repo<ControllerData> for ControllerDataRepo<'db> {
    fn add(&self, data: &ControllerData, db: &IsarDb) -> Result<(), Error> {
        let mut object_builder = db.get_object_builder(self.collection_name)?;
        data.write_with_object_builder(&mut object_builder);
        db.put(
            &self.collection_name,
            None,
            object_builder.finish().as_bytes(),
        )
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_all(&self, db: &IsarDb) -> Result<Vec<ControllerData>, Error> {
        let _routes_as_bytes = db.get_all_as_bytes(&self.collection_name)?;

        // TODO: not sure how to proceed to parse [u8] using the collection schema. didn't find examples in Isar
        unimplemented!()
    }
}

pub struct MockControllerDataRepo {}

impl Repo<ControllerData> for MockControllerDataRepo {
    fn add(&self, _object: &ControllerData, _db: &IsarDb) -> Result<(), Error> {
        Ok(())
    }

    fn get_all(&self, _db: &IsarDb) -> Result<Vec<ControllerData>, Error> {
        Ok(Vec::new())
    }
}
