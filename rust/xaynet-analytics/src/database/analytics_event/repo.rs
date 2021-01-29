use isar_core::error::IsarError;

use crate::database::{
    analytics_event::data_model::AnalyticsEvent,
    common::{IsarAdapter, Repo},
    instance::DbInstance,
};

pub struct AnalyticsEventRepo {
    collection_name: &'static str,
    db: &'static DbInstance,
}

impl AnalyticsEventRepo {
    pub fn new(collection_name: &'static str, db: &'static DbInstance) -> Self {
        Self {
            collection_name,
            db,
        }
    }
}

impl Repo<AnalyticsEvent> for AnalyticsEventRepo {
    fn add(&self, event: AnalyticsEvent) -> Result<(), IsarError> {
        let mut object_builder = match self.db.get_object_builder(self.collection_name) {
            Ok(object_builder) => object_builder,
            Err(e) => return Err(e),
        };
        event.write_with_object_builder(&mut object_builder);
        match self
            .db
            .put(self.collection_name, object_builder.finish().as_bytes())
        {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_all(&self) -> Result<Vec<AnalyticsEvent>, IsarError> {
        match self.db.get_all_as_bytes(self.collection_name) {
            Ok(events_as_bytes) => events_as_bytes,
            Err(e) => return Err(e),
        };

        // TODO: not sure how to proceed to parse [u8] using the collection schema. didn't find examples in Isar
        unimplemented!()
    }
}

pub struct MockAnalyticsEventRepo {}

impl Repo<AnalyticsEvent> for MockAnalyticsEventRepo {
    fn add(&self, _object: AnalyticsEvent) -> Result<(), IsarError> {
        Ok(())
    }

    fn get_all(&self) -> Result<Vec<AnalyticsEvent>, IsarError> {
        Ok(Vec::new())
    }
}
