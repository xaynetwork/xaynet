use anyhow::{Error, Result};

use crate::database::{
    analytics_event::data_model::AnalyticsEvent,
    common::{IsarAdapter, Repo},
    isar::IsarDb,
};

pub struct AnalyticsEventRepo<'db> {
    collection_name: &'db str,
    db: &'db IsarDb,
}

impl<'db> AnalyticsEventRepo<'db> {
    pub fn new(collection_name: &'db str, db: &'db IsarDb) -> Self {
        Self {
            collection_name,
            db,
        }
    }
}

impl<'screen, 'db> Repo<AnalyticsEvent<'screen>> for AnalyticsEventRepo<'db> {
    fn add(&self, event: AnalyticsEvent) -> Result<(), Error> {
        let mut object_builder = self.db.get_object_builder(self.collection_name)?;
        event.write_with_object_builder(&mut object_builder);
        self.db
            .put(
                &self.collection_name,
                None,
                object_builder.finish().as_bytes(),
            )
            .map(|_| ())
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_all(&self) -> Result<Vec<AnalyticsEvent<'screen>>, Error> {
        let _events_as_bytes = self.db.get_all_as_bytes(&self.collection_name)?;

        // TODO: not sure how to proceed to parse [u8] using the collection schema. didn't find examples in Isar
        // implement when possible: https://xainag.atlassian.net/browse/XN-1604
        unimplemented!()
    }
}

pub struct MockAnalyticsEventRepo {}

impl<'screen> Repo<AnalyticsEvent<'screen>> for MockAnalyticsEventRepo {
    fn add(&self, _object: AnalyticsEvent) -> Result<(), Error> {
        Ok(())
    }

    fn get_all(&self) -> Result<Vec<AnalyticsEvent<'screen>>, Error> {
        Ok(Vec::new())
    }
}
