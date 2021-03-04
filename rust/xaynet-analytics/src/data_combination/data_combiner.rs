//! Declaration and implementation of `DataCombiner`.

use anyhow::{Error, Result};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use std::iter::empty;

use crate::{
    data_combination::data_points::data_point::{
        CalcScreenActiveTime,
        CalcScreenEnterCount,
        CalcWasActiveEachPastPeriod,
        CalcWasActivePastNDays,
        DataPoint,
        DataPointMetadata,
        Period,
        PeriodUnit,
    },
    database::{
        analytics_event::data_model::AnalyticsEvent,
        screen_route::data_model::ScreenRoute,
    },
};

/// `DataCombiner` is responsible for instantiating the `DataPoint` variants. When it’s time to send the data to XayNet,
/// the `AnalyticsEvents` and `ScreenRoutes` are retrieved from the db (by the `AnalyticsController`) and passed to the `DataCombier`,
/// which then instantiates the various `DataPoint` variants and packs them in a `Vec`, which will be utilised by the `Sender`.
///
/// Possible improvements include:
/// - Move the `DataPointMetadatas` to a sort of config, and pass them to the `DataCombiner`.
/// - Turn `DataCombiner` into a trait on each `DataPoint`.
/// See: https://xainag.atlassian.net/browse/XN-1651
pub struct DataCombiner;

impl<'screen> DataCombiner {
    pub fn init_data_points(
        &self,
        events: &[AnalyticsEvent],
        screen_routes: &[ScreenRoute],
    ) -> Result<Vec<DataPoint>, Error> {
        let end_period = Utc::now();

        let one_day_period_metadata =
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        let was_active_each_period_metadatas = vec![
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 7), end_period),
            DataPointMetadata::new(Period::new(PeriodUnit::Weeks, 6), end_period),
            DataPointMetadata::new(Period::new(PeriodUnit::Months, 3), end_period),
        ];
        let was_active_past_days_metadatas = vec![
            one_day_period_metadata,
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 7), end_period),
            DataPointMetadata::new(Period::new(PeriodUnit::Days, 28), end_period),
        ];

        let data_points = empty::<DataPoint>()
            .chain(Self::init_screen_active_time_vars(
                one_day_period_metadata,
                events,
                screen_routes,
            ))
            .chain(Self::init_screen_enter_count_vars(
                one_day_period_metadata,
                events,
                screen_routes,
            ))
            .chain(Self::init_was_active_each_past_period_vars(
                was_active_each_period_metadatas,
                events,
            ))
            .chain(Self::init_was_active_past_n_days_vars(
                was_active_past_days_metadatas,
                events,
            ))
            .collect();
        Ok(data_points)
    }

    fn init_screen_active_time_vars(
        metadata: DataPointMetadata,
        events: &[AnalyticsEvent],
        screen_routes: &[ScreenRoute],
    ) -> Vec<DataPoint> {
        let mut screen_active_time_vars: Vec<DataPoint> = screen_routes
            .iter()
            .map(|route| {
                let events_this_route = Self::get_events_single_route(route, events);
                CalcScreenActiveTime::new(
                    metadata,
                    Self::filter_events_in_this_period(metadata, events_this_route.as_slice()),
                )
            })
            .map(DataPoint::ScreenActiveTime)
            .collect();
        screen_active_time_vars.push(DataPoint::ScreenActiveTime(CalcScreenActiveTime::new(
            metadata,
            Self::filter_events_in_this_period(metadata, events),
        )));
        screen_active_time_vars
    }

    fn init_screen_enter_count_vars(
        metadata: DataPointMetadata,
        events: &[AnalyticsEvent],
        screen_routes: &[ScreenRoute],
    ) -> Vec<DataPoint> {
        screen_routes
            .iter()
            .map(|route| {
                let events_this_route = Self::get_events_single_route(&route, events);
                CalcScreenEnterCount::new(
                    metadata,
                    Self::filter_events_in_this_period(metadata, events_this_route.as_slice()),
                )
            })
            .map(DataPoint::ScreenEnterCount)
            .collect()
    }

    fn init_was_active_each_past_period_vars(
        metadatas: Vec<DataPointMetadata>,
        events: &[AnalyticsEvent],
    ) -> Vec<DataPoint> {
        metadatas
            .iter()
            .map(|metadata| {
                let period_thresholds = (0..metadata.period.n)
                    .map(|i| Self::get_start_of_period(*metadata, Some(i)))
                    .collect();
                CalcWasActiveEachPastPeriod::new(
                    *metadata,
                    Self::filter_events_in_this_period(*metadata, events),
                    period_thresholds,
                )
            })
            .map(DataPoint::WasActiveEachPastPeriod)
            .collect()
    }

    fn init_was_active_past_n_days_vars(
        metadatas: Vec<DataPointMetadata>,
        events: &[AnalyticsEvent],
    ) -> Vec<DataPoint> {
        metadatas
            .iter()
            .map(|metadata| {
                CalcWasActivePastNDays::new(
                    *metadata,
                    Self::filter_events_in_this_period(*metadata, events),
                )
            })
            .map(DataPoint::WasActivePastNDays)
            .collect()
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn filter_events_in_this_period(
        metadata: DataPointMetadata,
        events: &[AnalyticsEvent],
    ) -> Vec<AnalyticsEvent> {
        let start_of_period = Self::get_start_of_period(metadata, None);
        Self::filter_events_before_end_of_period(metadata.end, events)
            .iter()
            .filter(|event| event.timestamp > start_of_period)
            .cloned()
            .collect()
    }

    fn get_start_of_period(
        metadata: DataPointMetadata,
        n_periods_override: Option<u32>,
    ) -> DateTime<Utc> {
        let n_periods = if let Some(n_periods) = n_periods_override {
            n_periods
        } else {
            metadata.period.n
        };
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
        end_of_period: DateTime<Utc>,
        events: &[AnalyticsEvent],
    ) -> Vec<AnalyticsEvent> {
        let midnight_end_of_period = get_midnight(end_of_period);
        events
            .iter()
            .filter(|event| event.timestamp < midnight_end_of_period)
            .cloned()
            .collect()
    }

    // TODO: return an iterator instead of Vec: https://xainag.atlassian.net/browse/XN-1517
    fn get_events_single_route(
        route: &ScreenRoute,
        all_events: &[AnalyticsEvent],
    ) -> Vec<AnalyticsEvent> {
        all_events
            .iter()
            .filter(|event| event.screen_route.as_ref() == Some(route))
            .cloned()
            .collect()
    }
}

fn get_midnight(timestamp: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::from_utc(
        NaiveDate::from_ymd(timestamp.year(), timestamp.month(), timestamp.day()).and_hms(0, 0, 0),
        Utc,
    )
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Duration, Utc};

    use crate::{
        data_combination::{
            data_combiner::{get_midnight, DataCombiner},
            data_points::data_point::{
                CalcScreenActiveTime,
                CalcScreenEnterCount,
                CalcWasActiveEachPastPeriod,
                CalcWasActivePastNDays,
                DataPoint,
                DataPointMetadata,
                Period,
                PeriodUnit,
            },
        },
        database::{
            analytics_event::data_model::{AnalyticsEvent, AnalyticsEventType},
            screen_route::data_model::ScreenRoute,
        },
    };

    #[test]
    fn test_init_screen_active_time_vars() {
        let end_period = DateTime::parse_from_rfc3339("2021-01-01T01:01:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        let screen_route = ScreenRoute::new("home_screen", end_period + Duration::days(1));
        let first_event = AnalyticsEvent::new(
            "test1",
            AnalyticsEventType::ScreenEnter,
            end_period - Duration::hours(12),
            Some(screen_route.clone()),
        );
        let all_events = vec![
            first_event.clone(),
            AnalyticsEvent::new(
                "test1",
                AnalyticsEventType::AppEvent,
                end_period - Duration::hours(13),
                None,
            ),
        ];
        let expected_output = vec![
            DataPoint::ScreenActiveTime(CalcScreenActiveTime::new(metadata, vec![first_event])),
            DataPoint::ScreenActiveTime(CalcScreenActiveTime::new(metadata, all_events.clone())),
        ];
        let actual_output =
            DataCombiner::init_screen_active_time_vars(metadata, &all_events, &[screen_route]);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_init_screen_enter_count_vars() {
        let end_period = DateTime::parse_from_rfc3339("2021-02-02T02:02:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        let screen_route = ScreenRoute::new("home_screen", end_period + Duration::days(1));
        let events = vec![AnalyticsEvent::new(
            "test1",
            AnalyticsEventType::ScreenEnter,
            end_period - Duration::hours(12),
            Some(screen_route.clone()),
        )];
        let expected_output = vec![DataPoint::ScreenEnterCount(CalcScreenEnterCount::new(
            metadata,
            events.clone(),
        ))];
        let actual_output =
            DataCombiner::init_screen_enter_count_vars(metadata, &events, &[screen_route]);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_init_was_active_each_past_period_vars() {
        let end_period = DateTime::parse_from_rfc3339("2021-03-03T03:03:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        let events = vec![AnalyticsEvent::new(
            "test1",
            AnalyticsEventType::AppEvent,
            end_period - Duration::hours(12),
            None,
        )];
        let period_thresholds = vec![get_midnight(end_period)];
        let expected_output = vec![DataPoint::WasActiveEachPastPeriod(
            CalcWasActiveEachPastPeriod::new(metadata, events.clone(), period_thresholds),
        )];
        let actual_output =
            DataCombiner::init_was_active_each_past_period_vars(vec![metadata], &events);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_init_was_active_past_n_days_vars() {
        let end_period = DateTime::parse_from_rfc3339("2021-04-04T04:04:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        let events = vec![AnalyticsEvent::new(
            "test1",
            AnalyticsEventType::AppEvent,
            end_period - Duration::hours(12),
            None,
        )];
        let expected_output = vec![DataPoint::WasActivePastNDays(CalcWasActivePastNDays::new(
            metadata,
            events.clone(),
        ))];
        let actual_output = DataCombiner::init_was_active_past_n_days_vars(vec![metadata], &events);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_filter_events_in_this_period() {
        let end_period = DateTime::parse_from_rfc3339("2021-05-05T05:05:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 3), end_period);
        let event_before = AnalyticsEvent::new(
            "test1",
            AnalyticsEventType::AppEvent,
            end_period - Duration::days(5),
            None,
        );
        let event_during = AnalyticsEvent::new(
            "test2",
            AnalyticsEventType::AppEvent,
            end_period - Duration::days(1),
            None,
        );
        let event_after = AnalyticsEvent::new(
            "test3",
            AnalyticsEventType::AppEvent,
            end_period + Duration::days(2),
            None,
        );
        let events = vec![event_before, event_during.clone(), event_after];
        let expected_output = vec![event_during];
        let actual_output = DataCombiner::filter_events_in_this_period(metadata, &events);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_get_start_of_period_one_day() {
        let end_period = DateTime::parse_from_rfc3339("2021-01-01T00:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 1), end_period);
        let expected_output = end_period - Duration::days(1);
        let actual_output = DataCombiner::get_start_of_period(metadata, None);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_get_start_of_period_one_day_with_override() {
        let end_period = DateTime::parse_from_rfc3339("2021-02-02T00:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Days, 2), end_period);
        let expected_output = end_period - Duration::days(1);
        let actual_output = DataCombiner::get_start_of_period(metadata, Some(1));
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_get_start_of_period_one_week() {
        let end_period = DateTime::parse_from_rfc3339("2021-03-03T00:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Weeks, 1), end_period);
        let expected_output = end_period - Duration::weeks(1);
        let actual_output = DataCombiner::get_start_of_period(metadata, None);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_get_start_of_period_one_month() {
        let end_period = DateTime::parse_from_rfc3339("2021-04-04T00:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let metadata = DataPointMetadata::new(Period::new(PeriodUnit::Months, 1), end_period);
        let expected_output = end_period - Duration::days(30);
        let actual_output = DataCombiner::get_start_of_period(metadata, None);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn text_filter_events_before_end_of_period() {
        let end_of_period = Utc::now();
        let event_before = AnalyticsEvent::new(
            "test1",
            AnalyticsEventType::AppEvent,
            end_of_period - Duration::days(1),
            None,
        );
        let event_after = AnalyticsEvent::new(
            "test2",
            AnalyticsEventType::AppEvent,
            end_of_period + Duration::days(1),
            None,
        );
        let events = vec![event_before.clone(), event_after];
        let expected_output = vec![event_before];
        let actual_output =
            DataCombiner::filter_events_before_end_of_period(end_of_period, &events);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_get_events_single_route() {
        let timestamp = Utc::now();
        let home_route = ScreenRoute::new("home_screen", timestamp + Duration::days(1));
        let other_route = ScreenRoute::new("other_screen", timestamp + Duration::days(2));
        let home_route_event = AnalyticsEvent::new(
            "test1",
            AnalyticsEventType::AppEvent,
            timestamp,
            Some(home_route.clone()),
        );
        let other_route_event = AnalyticsEvent::new(
            "test2",
            AnalyticsEventType::ScreenEnter,
            timestamp,
            Some(other_route),
        );
        let all_events = [home_route_event.clone(), other_route_event];
        let expected_output = vec![home_route_event];
        let actual_output = DataCombiner::get_events_single_route(&home_route, &all_events);
        assert_eq!(actual_output, expected_output);
    }

    #[test]
    fn test_get_midnight() {
        let timestamp = DateTime::parse_from_rfc3339("2021-01-01T21:21:21-02:00")
            .unwrap()
            .with_timezone(&Utc);
        let expected_output = DateTime::parse_from_rfc3339("2021-01-01T00:00:00-00:00")
            .unwrap()
            .with_timezone(&Utc);
        let actual_output = get_midnight(timestamp);
        assert_eq!(actual_output, expected_output);
    }
}
