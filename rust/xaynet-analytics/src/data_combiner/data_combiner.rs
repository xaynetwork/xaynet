use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use std::time::UNIX_EPOCH;

use crate::data_combiner::data_points::data_point::{
    CalculateDataPoints, DataPoints, DataPointMetadata, Period, PeriodUnit,
};
use crate::data_combiner::data_points::screen_active_time::ScreenActiveTime;
use crate::data_combiner::data_points::screen_enter_count::ScreenEnterCount;
use crate::data_combiner::data_points::was_active_each_past_period::WasActiveEachPastPeriod;
use crate::data_combiner::data_points::was_active_past_n_days::WasActivePastNDays;
use crate::repo::analytics_event::AnalyticsEvent;
use crate::repo::repo::Repository;

pub struct DataCombiner<R> {
    repo: Box<R>,
}

impl<R> DataCombiner<R>
where
    R: Repository,
{

    pub fn new(repo: Box<impl Repository>) -> DataCombiner<impl Repository> {
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
        let mut screen_active_time_vars: Vec<ScreenActiveTime> = Vec::new();
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        for screen_route in self.get_all_screen_routes().iter() {
            let events_this_route = self.get_events_single_route(screen_route.clone());
            let screen_active_time_this_route = ScreenActiveTime::new(
                metadata,
                self.filter_events_in_this_period(metadata, events_this_route),
            );
            screen_active_time_vars.push(screen_active_time_this_route);
        }
        let screen_active_time_all_routes = ScreenActiveTime::new(
            metadata,
            self.filter_events_in_this_period(metadata, self.get_all_events()),
        );
        screen_active_time_vars.push(screen_active_time_all_routes);
        screen_active_time_vars
    }

    fn init_screen_enter_count_vars(&self, end_period: DateTime<Utc>) -> Vec<ScreenEnterCount> {
        let mut screen_enter_count_vars: Vec<ScreenEnterCount> = Vec::new();
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        for screen_route in self.get_all_screen_routes().iter() {
            let events_this_route = self.get_events_single_route(screen_route.clone());
            let screen_enter_count_this_route = ScreenEnterCount::new(
                metadata,
                self.filter_events_in_this_period(metadata, events_this_route),
            );
            screen_enter_count_vars.push(screen_enter_count_this_route);
        }
        screen_enter_count_vars
    }

    fn init_was_active_each_past_period_vars(&self, end_period: DateTime<Utc>) -> Vec<WasActiveEachPastPeriod> {
        let metadatas = [
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 7), end_period),
            DataPointMetadata::new(Period::new(PeriodUnit::Weeks, 6), end_period),
            DataPointMetadata::new(Period::new(PeriodUnit::Months, 3), end_period),
        ];
        let mut was_active_each_past_periods: Vec<WasActiveEachPastPeriod> = Vec::new();
        for metadata in metadatas.iter() {
            let period_thresholds = (0..metadata.period.n)
                .map(|i| self.get_start_of_period(*metadata, i))
                .collect();
            let was_active_each_past_period = WasActiveEachPastPeriod::new(
                *metadata,
                self.filter_events_in_this_period(*metadata, self.get_all_events()),
                period_thresholds,
            );
            was_active_each_past_periods.push(was_active_each_past_period);
        }
        was_active_each_past_periods
    }

    fn get_all_events(&self) -> Vec<AnalyticsEvent> {
        self.repo.get_all_events()
    }

    /// TODO: don't use String here, handle via RouteController
    fn get_all_screen_routes(&self) -> Vec<String> {
        self.repo.get_all_routes()
    }

    fn filter_events_in_this_period(
        &self,
        metadata: DataPointMetadata,
        events: Vec<AnalyticsEvent>,
    ) -> Vec<AnalyticsEvent> {
        let start_of_period = self.get_start_of_period(metadata, metadata.period.n);
        let events_before_end_of_period =
            self.filter_events_before_end_of_period(metadata.end, events);
        if metadata.period.unit == PeriodUnit::Any {
            events_before_end_of_period
        } else {
            events_before_end_of_period
                .into_iter()
                .filter(|event| event.timestamp > start_of_period)
                .collect()
        }
    }

    fn get_start_of_period(&self, metadata: DataPointMetadata, n_periods: u32) -> DateTime<Utc> {
        let midnight_end_of_period = self.get_midnight(metadata.end);
        match metadata.period.unit {
            PeriodUnit::Days => midnight_end_of_period - Duration::days(n_periods as i64),
            PeriodUnit::Weeks => midnight_end_of_period - Duration::weeks(n_periods as i64),
            PeriodUnit::Months => apply_offset_months_to_timestamp(midnight_end_of_period, n_periods),
            PeriodUnit::Any => UNIX_EPOCH.into(),
        }
    }

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
