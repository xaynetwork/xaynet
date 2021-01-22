use anyhow::{Error, Result};

use crate::database::{
    analytics_event::data_model::AnalyticsEvent,
    common::{IsarAdapter, Repo},
    isar::IsarDb,
};

pub struct AnalyticsEventRepo {
    collection_name: &'static str,
    db: &'static IsarDb,
}

impl AnalyticsEventRepo {
    pub fn new(collection_name: &'static str, db: &'static IsarDb) -> Self {
        Self {
            collection_name,
            db,
        }
    }
}

impl Repo<AnalyticsEvent> for AnalyticsEventRepo {
    fn add(&self, event: AnalyticsEvent) -> Result<(), Error> {
        let mut object_builder = self.db.get_object_builder(self.collection_name)?;
        event.write_with_object_builder(&mut object_builder);
        self.db
            .put(self.collection_name, object_builder.finish().as_bytes())
            .map(|_| ())
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_all(&self) -> Result<Vec<AnalyticsEvent>, Error> {
        let _events_as_bytes = self.db.get_all_as_bytes(self.collection_name)?;

        // TODO: not sure how to proceed to parse [u8] using the collection schema. didn't find examples in Isar
        unimplemented!()
    }
}

pub struct MockAnalyticsEventRepo {}

impl Repo<AnalyticsEvent> for MockAnalyticsEventRepo {
    fn add(&self, _object: AnalyticsEvent) -> Result<(), Error> {
        Ok(())
    }

    fn get_all(&self) -> Result<Vec<AnalyticsEvent>, Error> {
        Ok(Vec::new())
    }
}
