use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};

use std::iter::empty;

use crate::{
    data_combination::data_points::data_point::{
        CalcScreenActiveTime, CalcScreenEnterCount, CalcWasActiveEachPastPeriod,
        CalcWasActivePastNDays, DataPoint, DataPointMetadata, Period, PeriodUnit,
    },
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

    pub fn init_data_points(&self) -> Vec<DataPoint> {
        let end_period = Utc::now();
        empty::<DataPoint>()
            .chain(self.init_was_active_past_n_days_vars(end_period))
            .chain(self.init_screen_active_time_vars(end_period))
            .chain(self.init_screen_enter_count_vars(end_period))
            .chain(self.init_was_active_each_past_period_vars(end_period))
            .collect()
    }

    fn init_was_active_past_n_days_vars(&self, end_period: DateTime<Utc>) -> Vec<DataPoint> {
        [
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period),
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 7), end_period),
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 28), end_period),
        ]
        .iter()
        .map(|metadata| {
            CalcWasActivePastNDays::new(
                *metadata,
                self.filter_events_in_this_period(*metadata, self.get_all_events()),
            )
        })
        .map(|data| DataPoint::WasActivePastNDays(data))
        .collect()
    }

    fn init_screen_active_time_vars(&self, end_period: DateTime<Utc>) -> Vec<DataPoint> {
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        let mut screen_active_time_vars: Vec<DataPoint> = self
            .get_all_screen_routes()
            .iter()
            .map(|route| {
                let events_this_route = self.get_events_single_route(route.clone());
                CalcScreenActiveTime::new(
                    metadata,
                    self.filter_events_in_this_period(metadata, events_this_route),
                )
            })
            .map(|data| DataPoint::ScreenActiveTime(data))
            .collect();
        screen_active_time_vars.push(DataPoint::ScreenActiveTime(CalcScreenActiveTime::new(
            metadata,
            self.filter_events_in_this_period(metadata, self.get_all_events()),
        )));
        screen_active_time_vars
    }

    fn init_screen_enter_count_vars(&self, end_period: DateTime<Utc>) -> Vec<DataPoint> {
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        self.get_all_screen_routes()
            .iter()
            .map(|route| {
                let events_this_route = self.get_events_single_route(route.clone());
                CalcScreenEnterCount::new(
                    metadata,
                    self.filter_events_in_this_period(metadata, events_this_route),
                )
            })
            .map(|data| DataPoint::ScreenEnterCount(data))
            .collect()
    }

    fn init_was_active_each_past_period_vars(&self, end_period: DateTime<Utc>) -> Vec<DataPoint> {
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
            CalcWasActiveEachPastPeriod::new(
                *metadata,
                self.filter_events_in_this_period(*metadata, self.get_all_events()),
                period_thresholds,
            )
        })
        .map(|data| DataPoint::WasActiveEachPastPeriod(data))
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
        self.filter_events_before_end_of_period(metadata.end, events)
            .into_iter()
            .filter(|event| event.timestamp > start_of_period)
            .collect()
    }

    fn get_start_of_period(&self, metadata: DataPointMetadata, n_periods: u32) -> DateTime<Utc> {
        let avg_days_per_month = 365.0 / 12.0;
        let midnight_end_of_period = get_midnight(metadata.end);
        match metadata.period.unit {
            PeriodUnit::Days => midnight_end_of_period - Duration::days(n_periods as i64),
            PeriodUnit::Weeks => midnight_end_of_period - Duration::weeks(n_periods as i64),
            PeriodUnit::Months => {
                midnight_end_of_period
                    - Duration::days((n_periods as f64 * avg_days_per_month) as i64)
            }
        }
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn filter_events_before_end_of_period(
        &self,
        end_of_period: DateTime<Utc>,
        events: Vec<AnalyticsEvent>,
    ) -> Vec<AnalyticsEvent> {
        let midnight_end_of_period = get_midnight(end_of_period);
        events
            .into_iter()
            .filter(|event| event.timestamp < midnight_end_of_period)
            .collect()
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_events_single_route(&self, route: String) -> Vec<AnalyticsEvent> {
        self.get_all_events()
            .into_iter()
            .filter(|event| event.screen_route.as_ref().unwrap() == &route)
            .collect()
    }
}

fn get_midnight(timestamp: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::from_utc(
        NaiveDate::from_ymd(timestamp.year(), timestamp.month(), timestamp.day()).and_hms(0, 0, 0),
        Utc,
    )
}
