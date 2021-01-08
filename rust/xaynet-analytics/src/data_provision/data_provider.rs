use crate::data_provision::analytics_event::AnalyticsEvent;

pub trait DataProvider: Sized {
    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_all_events(&self) -> Vec<AnalyticsEvent>;

    fn get_all_routes(&self) -> Vec<String>;
}

pub struct AnalyticsDataProvider {}

impl DataProvider for AnalyticsDataProvider {
    fn get_all_events(&self) -> Vec<AnalyticsEvent> {
        // TODO: https://xainag.atlassian.net/browse/XN-1409
        unimplemented!()
    }

    fn get_all_routes(&self) -> Vec<String> {
        // TODO: https://xainag.atlassian.net/browse/XN-1409
        unimplemented!()
    }
}

pub struct MockAnalyticsDataProvider {}

impl DataProvider for MockAnalyticsDataProvider {
    fn get_all_events(&self) -> Vec<AnalyticsEvent> {
        // TODO will return hardcoded list of events for testings
        unimplemented!()
    }

    fn get_all_routes(&self) -> Vec<String> {
        // TODO will return hardcoded list of routes for testing
        unimplemented!()
    }
}
