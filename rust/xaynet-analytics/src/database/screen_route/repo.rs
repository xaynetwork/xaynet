use anyhow::{Error, Result};

use crate::database::{
    common::{IsarAdapter, Repo},
    isar::IsarDb,
    screen_route::data_model::ScreenRoute,
};

pub struct ScreenRouteRepo<'db> {
    collection_name: &'db str,
    db: &'db IsarDb,
}

impl<'db> ScreenRouteRepo<'db> {
    pub fn new(collection_name: &'db str, db: &'db IsarDb) -> Self {
        Self {
            collection_name,
            db,
        }
    }
}

impl<'db> Repo<&'db mut ScreenRoute> for ScreenRouteRepo<'db> {
    fn add(&self, route: &'db mut ScreenRoute) -> Result<(), Error> {
        let mut object_builder = self.db.get_object_builder(&self.collection_name)?;
        route.write_with_object_builder(&mut object_builder);
        let object_id = self
            .db
            .put(&self.collection_name, object_builder.finish().as_bytes())?;
        route.object_id = Some(object_id);
        Ok(())
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_all(&self) -> Result<Vec<&'db mut ScreenRoute>, Error> {
        let _routes_as_bytes = self.db.get_all_as_bytes(&self.collection_name)?;

        // TODO: not sure how to proceed to parse [u8] using the collection schema. didn't find examples in Isar
        unimplemented!()
    }
}

pub struct MockScreenRouteRepo {}

impl Repo<ScreenRoute> for MockScreenRouteRepo {
    fn add(&self, _object: ScreenRoute) -> Result<(), Error> {
        Ok(())
    }

    fn get_all(&self) -> Result<Vec<ScreenRoute>, Error> {
        Ok(Vec::new())
    }
}
