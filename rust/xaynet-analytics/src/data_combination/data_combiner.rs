use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};

use crate::{
    data_combination::data_points::data_point::{CalculateDataPoints, DataPoints, DataPointMetadata, Period, PeriodUnit},
    data_combination::data_points::screen_active_time::ScreenActiveTime,
    data_combination::data_points::screen_enter_count::ScreenEnterCount,
    data_combination::data_points::was_active_each_past_period::WasActiveEachPastPeriod,
    data_combination::data_points::was_active_past_n_days::WasActivePastNDays,
    data_provision::analytics_event::AnalyticsEvent,
    data_provision::data_provider::DataProvider,
};

pub struct DataCombiner<R> {
    repo: Box<R>,
}

impl<R> DataCombiner<R>
where
    R: DataProvider,
{

    pub fn new(repo: Box<impl DataProvider>) -> DataCombiner<impl DataProvider> {
        DataCombiner { repo }
    }

    pub fn init_data_points(&self) -> Vec<DataPoints> {
        let end_period = Utc::now();
        [
            self.to_data_points(self.init_was_active_past_n_days_vars(end_period)),
            self.to_data_points(self.init_screen_active_time_vars(end_period)),
            self.to_data_points(self.init_screen_enter_count_vars(end_period)),
            self.to_data_points(self.init_was_active_each_past_period_vars(end_period)),
        ]
        .concat()
    }

    fn to_data_points(&self, calculables: Vec<impl CalculateDataPoints>) -> Vec<DataPoints> {
        calculables
            .into_iter()
            .map(|calculable| DataPoints::new(calculable.metadata(), calculable.calculate()))
            .collect()
    }

    fn init_was_active_past_n_days_vars(&self, end_period: DateTime<Utc>) -> Vec<impl CalculateDataPoints> {
        [
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period),
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 7), end_period),
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 28), end_period),
        ]
            .iter()
            .map(|metadata| {
                WasActivePastNDays::new(
                    *metadata,
                    self.filter_events_in_this_period(*metadata, self.get_all_events()),
                )
            })
            .collect()
    }

    fn init_screen_active_time_vars(&self, end_period: DateTime<Utc>) -> Vec<impl CalculateDataPoints> {
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        let mut screen_active_time_vars: Vec<ScreenActiveTime> = self.get_all_screen_routes()
            .iter()
            .map(|route| {
                let events_this_route = self.get_events_single_route(route.clone());
                ScreenActiveTime::new(
                    metadata,
                    self.filter_events_in_this_period(metadata, events_this_route),
                )
            })
            .collect();
        screen_active_time_vars.push(ScreenActiveTime::new(
            metadata,
            self.filter_events_in_this_period(metadata, self.get_all_events()),
        ));
        screen_active_time_vars
    }

    fn init_screen_enter_count_vars(&self, end_period: DateTime<Utc>) -> Vec<ScreenEnterCount> {
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        self.get_all_screen_routes()
            .iter()
            .map(|route| {
                let events_this_route = self.get_events_single_route(route.clone());
                ScreenEnterCount::new(
                    metadata,
                    self.filter_events_in_this_period(metadata, events_this_route),
                )
            })
            .collect()
    }

    fn init_was_active_each_past_period_vars(&self, end_period: DateTime<Utc>) -> Vec<WasActiveEachPastPeriod> {
        [
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 7), end_period),
            DataPointMetadata::new(Period::new(PeriodUnit::Weeks, 6), end_period),
            DataPointMetadata::new(Period::new(PeriodUnit::Months, 3), end_period),
        ]
            .iter()
            .map(|metadata| {
                let period_thresholds = (0..metadata.period.n)
                    .map(|i| self.get_start_of_period(*metadata, i))
                    .collect();
                WasActiveEachPastPeriod::new(
                    *metadata,
                    self.filter_events_in_this_period(*metadata, self.get_all_events()),
                    period_thresholds,
                )
            })
            .collect()
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_all_events(&self) -> Vec<AnalyticsEvent> {
        self.repo.get_all_events()
    }

    /// TODO: don't use String here, handle via RouteController
    fn get_all_screen_routes(&self) -> Vec<String> {
        self.repo.get_all_routes()
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn filter_events_in_this_period(
        &self,
        metadata: DataPointMetadata,
        events: Vec<AnalyticsEvent>,
    ) -> Vec<AnalyticsEvent> {
        let start_of_period = self.get_start_of_period(metadata, metadata.period.n);
        let events_before_end_of_period =
            self.filter_events_before_end_of_period(metadata.end, events);
        events_before_end_of_period
            .into_iter()
            .filter(|event| event.timestamp > start_of_period)
            .collect()

    }

    fn get_start_of_period(&self, metadata: DataPointMetadata, n_periods: u32) -> DateTime<Utc> {
        let midnight_end_of_period = self.get_midnight(metadata.end);
        match metadata.period.unit {
            PeriodUnit::Days => midnight_end_of_period - Duration::days(n_periods as i64),
            PeriodUnit::Weeks => midnight_end_of_period - Duration::weeks(n_periods as i64),
            PeriodUnit::Months => apply_offset_months_to_timestamp(midnight_end_of_period, n_periods),
        }
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn filter_events_before_end_of_period(
        &self,
        end_of_period: DateTime<Utc>,
        events: Vec<AnalyticsEvent>,
    ) -> Vec<AnalyticsEvent> {
        let midnight_end_of_period = self.get_midnight(end_of_period);
        events
            .into_iter()
            .filter(|event| event.timestamp < midnight_end_of_period)
            .collect()
    }

    fn get_midnight(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        apply_offset_months_to_timestamp(timestamp, 0)
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_events_single_route(&self, route: String) -> Vec<AnalyticsEvent> {
        self.get_all_events()
            .into_iter()
            .filter(|event| event.screen_route.as_ref().unwrap() == &route)
            .collect()
    }
}

fn apply_offset_months_to_timestamp(timestamp: DateTime<Utc>, n_months_offset: u32) -> DateTime<Utc> {
    let naive_offest_timestamp = NaiveDate::from_ymd(
        timestamp.year(),
        timestamp.month() - n_months_offset as u32,
        timestamp.day(),
    )
    .and_hms(0, 0, 0);
    DateTime::from_utc(naive_offest_timestamp, Utc)
}
