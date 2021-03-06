//! In this file the `AnalyticsController` is defined.

use anyhow::{anyhow, Error, Result};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};

use crate::{
    data_combination::data_combiner::DataCombiner,
    database::{
        analytics_event::{
            adapter::AnalyticsEventAdapter,
            data_model::{AnalyticsEvent, AnalyticsEventType},
        },
        common::{CollectionNames, Repo, SchemaGenerator},
        controller_data::{adapter::ControllerDataAdapter, data_model::ControllerData},
        isar::IsarDb,
        screen_route::{adapter::ScreenRouteAdapter, data_model::ScreenRoute},
    },
    sender::Sender,
};

/// The `AnalyticsController` is the core component of the library. It exposes public functions to the FFI wrapper, and it’s responsible for:
/// - Instantiating the other necessary components (`DataCombiner`, `Sender` and `IsarDb`)
/// - Receiving incoming data recorded by the mobile framework (via FFI of course) and saving them to the db via `IsarDb`.
/// - Checking if the library needs to send data to the XayNet coordinator via `Sender`.
/// - Holding some simple state (`self.is_charging`, `self.is_connected_to_wifi`) so that it knows whether it’s appropriate to send data to XayNet.
///
/// ## Arguments
///
/// * `db` - Singleton instance of `IsarDb`, used to operate with the database.
/// * `is_charging` - Boolean flag representing whether the phone is currently charging or not.
/// * `is_connected_to_wifi` - Boolean flag representing whether the phone is currently connected to the wifi or not.
/// * `last_time_data_sent` - Timestamp representing when analytics data was last sent to the coordinator. If `None`, data was never sent before.
/// * `combiner` - `DataCombiner` component responsible for calculating `DataPoints` based on `AnalyticsEvents` and `ScreenRoutes`.
/// * `sender` - `Sender` component responsible for preparing the message to be sent to the coordinator for aggregation.
/// * `send_frequency_hours` - `Duration` in hours representing periods within which we want to send data to the coordinator only once.
struct AnalyticsController {
    db: IsarDb,
    is_charging: bool,
    is_connected_to_wifi: bool,
    last_time_data_sent: Option<DateTime<Utc>>,
    combiner: DataCombiner,
    sender: Sender,
    send_frequency_hours: Duration,
}

// TODO: remove allow dead code when AnalyticsController is integrated with FFI layer: https://xainag.atlassian.net/browse/XN-1415
#[allow(dead_code)]
impl AnalyticsController {
    const MAX_SEND_FREQUENCY_HOURS: u8 = 24;

    pub fn init(
        path: String,
        is_charging: bool,
        is_connected_to_wifi: bool,
        input_send_frequency_hours: Option<u8>,
    ) -> Result<Self, Error> {
        let schemas = vec![
            AnalyticsEventAdapter::get_schema(&CollectionNames::ANALYTICS_EVENTS)?,
            ControllerDataAdapter::get_schema(&CollectionNames::CONTROLLER_DATA)?,
            ScreenRouteAdapter::get_schema(&CollectionNames::SCREEN_ROUTES)?,
        ];
        let db = IsarDb::new(&path, schemas)?;
        let last_time_data_sent = Self::get_last_time_data_sent(&db)?;
        let send_frequency_hours = Self::validate_send_frequency(input_send_frequency_hours)?;

        Ok(AnalyticsController {
            db,
            is_charging,
            is_connected_to_wifi,
            last_time_data_sent,
            combiner: DataCombiner,
            sender: Sender,
            send_frequency_hours,
        })
    }

    pub fn dispose(self) -> Result<(), Error> {
        self.db.dispose()
    }

    pub fn save_analytics_event(
        &self,
        name: &str,
        event_type: AnalyticsEventType,
        timestamp: DateTime<Utc>,
        option_screen_route_name: Option<&str>,
    ) -> Result<(), Error> {
        let option_screen_route = option_screen_route_name
            .map(|screen_route_name| self.add_screen_route_if_new(screen_route_name, timestamp))
            .transpose()?;
        let event = AnalyticsEvent::new(name, event_type, timestamp, option_screen_route);
        event.save(&self.db, &CollectionNames::ANALYTICS_EVENTS)?;
        Ok(())
    }

    pub fn change_connectivity_status(&mut self) {
        self.is_connected_to_wifi = !self.is_connected_to_wifi;
    }

    pub fn change_state_of_charge(&mut self) {
        self.is_charging = !self.is_charging;
    }

    pub fn maybe_send_data(&mut self) -> Result<(), Error> {
        if self.should_send_data() {
            self.send_data()
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    fn db(&self) -> &IsarDb {
        &self.db
    }

    /// Check whether `input_send_frequency_hours` is at most `MAX_SEND_FREQUENCY_HOURS`, otherwise return an `Error`.
    /// If it's lower, return a `Duration`.
    /// If it's `None`, assign `Self::MAX_SEND_FREQUENCY_HOURS` and turn it into a `Duration` as well.
    fn validate_send_frequency(input_send_frequency_hours: Option<u8>) -> Result<Duration, Error> {
        let send_frequency_hours =
            input_send_frequency_hours.unwrap_or(Self::MAX_SEND_FREQUENCY_HOURS);
        if send_frequency_hours > Self::MAX_SEND_FREQUENCY_HOURS {
            Err(anyhow!(
                "input_send_frequency_hours must be between 0 and {}",
                Self::MAX_SEND_FREQUENCY_HOURS
            ))
        } else {
            Ok(Duration::hours(send_frequency_hours as i64))
        }
    }

    fn should_send_data(&self) -> bool {
        let can_send_data = self.is_charging && self.is_connected_to_wifi;
        can_send_data && !self.did_send_already_in_this_period()
    }

    /// Check whether the new incoming `screen_route_name` already exists in the `ScreenRoutes` saved to the db.
    /// If it exists, return the existing `ScreenRoute` object from the db.
    /// If it doesn't exist, create the new `ScreenRoute` object, save it to db, and return a clone of it.
    fn add_screen_route_if_new(
        &self,
        screen_route_name: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<ScreenRoute, Error> {
        let existing_screen_routes =
            ScreenRoute::get_all(&self.db, &CollectionNames::SCREEN_ROUTES)?;
        if let Some(existing_screen_route) = existing_screen_routes
            .into_iter()
            .find(|existing_route| existing_route.name == screen_route_name)
        {
            Ok(existing_screen_route)
        } else {
            let screen_route = ScreenRoute::new(screen_route_name, timestamp);
            screen_route
                .clone()
                .save(&self.db, &CollectionNames::SCREEN_ROUTES)?;
            Ok(screen_route)
        }
    }

    fn get_last_time_data_sent(db: &IsarDb) -> Result<Option<DateTime<Utc>>, Error> {
        Ok(
            ControllerData::get_all(db, &CollectionNames::CONTROLLER_DATA)?
                .last()
                .map(|data| data.time_data_sent),
        )
    }

    /// This method implements a sliding 'time window' of `self.send_frequency_hours` duration, to check whether we have
    /// already sent data in the current window, or not.
    ///
    /// An alternative implementation could be based on simply checking whether:
    /// `last_time_data_sent > Utc::now() - self.send_frequency_hours`
    ///
    /// In the current implementation, it might be easier to then group the aggregated data on the coordinator side,
    /// to then be displayed in the UI, especially if `self.send_frequency_hours == Duration::hours(24)`.
    ///
    /// The more dynamic approach however implies that if, for example, `self.send_frequency_hours == Duration::hours(6)`,
    /// and the last time we sent the data was at 5AM, we would be able to send again at 7AM, while with the simpler solution
    /// we wouldn't be able to send again until 11AM.
    ///
    /// The correct approach to be chosen should very much depend on the amount of data available for aggregation,
    /// and it's possible that `MAX_SEND_FREQUENCY_HOURS` should be increased to more than 24.
    /// In that case, this function below will need to be reworked, because it' coupled with `MAX_SEND_FREQUENCY_HOURS` being 24.
    ///
    /// Only once it's more clear how the aggregation will work on the coordinator side, there will be more information
    /// to decide the approach here.
    fn did_send_already_in_this_period(&self) -> bool {
        self.last_time_data_sent
            .map(|last_time_data_sent| {
                let now = Utc::now();
                let start_of_day: DateTime<Utc> = DateTime::from_utc(
                    NaiveDate::from_ymd(now.year(), now.month(), now.day()).and_hms(0, 0, 0),
                    Utc,
                );
                let mut end_of_current_period = start_of_day + self.send_frequency_hours;
                while now > end_of_current_period {
                    end_of_current_period = end_of_current_period + self.send_frequency_hours;
                }
                let start_of_current_period = end_of_current_period - self.send_frequency_hours;
                last_time_data_sent > start_of_current_period
            })
            .unwrap_or(false)
    }

    /// Retrive all `AnalyticsEvents` and `ScreenRoutes` from the db and pass them to the `DataCombiner`.
    /// The `DataCombiner` will init all `DataPoints` and pack them in a `Vec<DataPoint>`, which will be the input to the `Sender`.
    /// After that, save the new time_data_sent inside `ControllerData`, and cache it in `self.last_time_data_sent`
    fn send_data(&mut self) -> Result<(), Error> {
        let events = AnalyticsEvent::get_all(&self.db, &CollectionNames::ANALYTICS_EVENTS)?;
        let screen_routes = ScreenRoute::get_all(&self.db, &CollectionNames::SCREEN_ROUTES)?;
        let time_data_sent = Utc::now();
        self.sender
            .send(self.combiner.init_data_points(&events, &screen_routes)?)
            .and_then(|_| {
                ControllerData::new(time_data_sent)
                    .save(&self.db, &CollectionNames::CONTROLLER_DATA)
            })
            .map(|_| self.last_time_data_sent = Some(time_data_sent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{env, fs, path::PathBuf};

    fn get_path(test_name: &str) -> PathBuf {
        let temp_dir = env::temp_dir();
        temp_dir.join(test_name)
    }

    fn get_controller(
        test_name: &str,
        input_send_data_frequency: Option<u8>,
    ) -> AnalyticsController {
        let path_buf = get_path(test_name);
        let path = path_buf.to_str().unwrap().to_string();
        if !path_buf.exists() {
            fs::create_dir(path.clone()).unwrap();
        }
        AnalyticsController::init(path, true, true, input_send_data_frequency).unwrap()
    }

    fn remove_dir(test_name: &str) {
        let path = get_path(test_name);
        std::fs::remove_dir_all(path).unwrap();
    }

    fn cleanup(controller: AnalyticsController, test_name: &str) {
        remove_dir(test_name);
        controller.dispose().unwrap();
    }

    #[test]
    fn test_dispose() {
        let test_name = "test_dispose";
        let controller = get_controller(test_name, None);
        assert!(controller.dispose().is_ok());
        remove_dir(test_name);
    }

    #[test]
    fn test_save_analytics_event_no_screen_route() {
        let test_name = "test_save_analytics_event_no_screen_route";
        let controller = get_controller(test_name, None);
        let name = "test";
        let event_type = AnalyticsEventType::AppEvent;
        let timestamp = DateTime::parse_from_rfc3339("2021-01-01T01:01:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        let existing_analytics_events =
            AnalyticsEvent::get_all(controller.db(), CollectionNames::ANALYTICS_EVENTS).unwrap();
        assert!(existing_analytics_events.is_empty());
        assert!(controller
            .save_analytics_event(name, event_type, timestamp, None)
            .is_ok());

        let analytics_event = AnalyticsEvent::new(name, event_type, timestamp, None);
        let all_analytics_events =
            AnalyticsEvent::get_all(controller.db(), CollectionNames::ANALYTICS_EVENTS).unwrap();
        assert_eq!(all_analytics_events.len(), 1);
        assert_eq!(all_analytics_events.first(), Some(&analytics_event));

        cleanup(controller, test_name);
    }

    #[test]
    fn test_save_analytics_event_with_screen_route() {
        let test_name = "test_save_analytics_event_with_screen_route";
        let controller = get_controller(test_name, None);
        let name = "test";
        let event_type = AnalyticsEventType::ScreenEnter;
        let timestamp = DateTime::parse_from_rfc3339("2021-01-01T01:01:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        let screen_route_name = "route";
        assert!(controller
            .save_analytics_event(name, event_type, timestamp, Some(screen_route_name))
            .is_ok());

        let screen_route = ScreenRoute::new(screen_route_name, timestamp);
        let analytics_event = AnalyticsEvent::new(name, event_type, timestamp, Some(screen_route));
        let all_analytics_events =
            AnalyticsEvent::get_all(controller.db(), CollectionNames::ANALYTICS_EVENTS).unwrap();
        assert_eq!(all_analytics_events.len(), 1);
        assert_eq!(all_analytics_events.first(), Some(&analytics_event));

        cleanup(controller, test_name);
    }

    #[test]
    fn test_change_connectivity_status() {
        let test_name = "test_change_connectivity_status";
        let mut controller = get_controller(test_name, None);
        assert!(controller.is_connected_to_wifi);
        controller.change_connectivity_status();
        assert!(!controller.is_connected_to_wifi);

        cleanup(controller, test_name);
    }

    #[test]
    fn test_change_state_of_charge() {
        let test_name = "test_change_state_of_charge";
        let mut controller = get_controller(test_name, None);
        assert!(controller.is_charging);
        controller.change_state_of_charge();
        assert!(!controller.is_charging);

        cleanup(controller, test_name);
    }

    #[test]
    fn test_validate_send_data_frequency_when_none() {
        assert_eq!(
            AnalyticsController::validate_send_frequency(None).unwrap(),
            Duration::hours(AnalyticsController::MAX_SEND_FREQUENCY_HOURS as i64)
        )
    }

    #[test]
    fn test_validate_send_data_frequency_when_more_than_24() {
        assert!(AnalyticsController::validate_send_frequency(Some(25)).is_err());
    }

    #[test]
    fn test_validate_send_data_frequency_when_less_than_24() {
        assert_eq!(
            AnalyticsController::validate_send_frequency(Some(6)).unwrap(),
            Duration::hours(6)
        )
    }

    #[test]
    fn test_validate_send_data_frequency_when_0() {
        assert_eq!(
            AnalyticsController::validate_send_frequency(Some(0)).unwrap(),
            Duration::hours(0)
        )
    }

    #[test]
    fn test_add_screen_route_if_new_with_new_route() {
        let test_name = "test_add_screen_route_if_new_with_new_route";
        let controller = get_controller(test_name, None);
        let screen_route_name = "route";
        let timestamp = DateTime::parse_from_rfc3339("2021-01-01T01:01:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        let screen_route = ScreenRoute::new(screen_route_name, timestamp);
        let existing_screen_routes =
            ScreenRoute::get_all(controller.db(), CollectionNames::SCREEN_ROUTES).unwrap();
        assert!(existing_screen_routes.is_empty());
        assert_eq!(
            controller
                .add_screen_route_if_new(screen_route_name, timestamp)
                .unwrap(),
            screen_route
        );

        let retrieved_screen_routes =
            ScreenRoute::get_all(controller.db(), CollectionNames::SCREEN_ROUTES).unwrap();
        assert_eq!(retrieved_screen_routes.len(), 1);
        assert_eq!(retrieved_screen_routes.first(), Some(&screen_route));

        cleanup(controller, test_name);
    }

    #[test]
    fn test_add_screen_route_if_new_without_new_route() {
        let test_name = "test_add_screen_route_if_new_without_new_route";
        let controller = get_controller(test_name, None);
        let screen_route_name = "route";
        let first_timestamp = DateTime::parse_from_rfc3339("2021-01-01T01:01:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        let first_screen_route = ScreenRoute::new(screen_route_name, first_timestamp);
        assert!(controller
            .add_screen_route_if_new(screen_route_name, first_timestamp)
            .is_ok());

        // if we call controller.add_screen_route_if_new() with the same screen_route_name, but a new_timestamp,
        // we expect to get the first_screen_route back, with the first_timestamp
        let new_timestamp = DateTime::parse_from_rfc3339("2021-02-02T02:02:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            controller
                .add_screen_route_if_new(screen_route_name, new_timestamp)
                .unwrap(),
            first_screen_route
        );

        let retrieved_screen_routes =
            ScreenRoute::get_all(controller.db(), CollectionNames::SCREEN_ROUTES).unwrap();
        assert_eq!(retrieved_screen_routes.len(), 1);
        assert_eq!(retrieved_screen_routes.first(), Some(&first_screen_route));

        cleanup(controller, test_name);
    }

    #[test]
    fn test_get_last_time_data_sent() {
        let test_name = "test_get_last_time_data_sent_is_none";
        let controller = get_controller(test_name, None);

        let last_time_data_sent = AnalyticsController::get_last_time_data_sent(controller.db());
        assert!(last_time_data_sent.is_ok());
        assert!(last_time_data_sent.unwrap().is_none());

        let timestamp = DateTime::parse_from_rfc3339("2021-03-03T03:03:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        let controller_data = ControllerData::new(timestamp);
        let existing_controller_data =
            ControllerData::get_all(controller.db(), CollectionNames::CONTROLLER_DATA).unwrap();
        assert!(existing_controller_data.is_empty());
        assert!(controller_data
            .save(controller.db(), CollectionNames::CONTROLLER_DATA)
            .is_ok());

        let last_time_data_sent = AnalyticsController::get_last_time_data_sent(controller.db());
        assert!(last_time_data_sent.is_ok());
        assert_eq!(last_time_data_sent.unwrap(), Some(timestamp));

        cleanup(controller, test_name);
    }

    #[test]
    fn test_did_send_already_in_this_period_never_sent_before() {
        let test_name = "test_did_send_already_in_this_period_never_sent_before";
        let controller = get_controller(test_name, Some(24));
        assert!(!controller.did_send_already_in_this_period());

        cleanup(controller, test_name);
    }

    #[test]
    fn test_did_send_already_in_this_period_inside_24h() {
        let test_name = "test_did_send_already_in_this_period_inside_24h";
        let initial_controller = get_controller(test_name, Some(24));

        let timestamp = Utc::now();
        let controller_data = ControllerData::new(timestamp);
        assert!(controller_data
            .save(initial_controller.db(), CollectionNames::CONTROLLER_DATA)
            .is_ok());

        // init controller again, to read self.last_time_data_sent from db
        assert!(initial_controller.dispose().is_ok());
        let controller = get_controller(test_name, Some(24));
        assert!(controller.did_send_already_in_this_period());

        cleanup(controller, test_name);
    }

    #[test]
    fn test_did_send_already_in_this_period_outside_24h() {
        let test_name = "test_did_send_already_in_this_period_outside_24h";
        let initial_controller = get_controller(test_name, Some(24));

        let timestamp = Utc::now() - Duration::hours(25);
        let controller_data = ControllerData::new(timestamp);
        assert!(controller_data
            .save(initial_controller.db(), CollectionNames::CONTROLLER_DATA)
            .is_ok());

        // init controller again, to read self.last_time_data_sent from db
        assert!(initial_controller.dispose().is_ok());
        let controller = get_controller(test_name, Some(24));
        assert!(!controller.did_send_already_in_this_period());

        cleanup(controller, test_name);
    }

    #[test]
    fn test_did_send_already_in_this_period_inside_12h() {
        let test_name = "test_did_send_already_in_this_period_inside_12h";
        let initial_controller = get_controller(test_name, Some(12));

        let timestamp = Utc::now();
        let controller_data = ControllerData::new(timestamp);
        assert!(controller_data
            .save(initial_controller.db(), CollectionNames::CONTROLLER_DATA)
            .is_ok());

        // init controller again, to read self.last_time_data_sent from db
        assert!(initial_controller.dispose().is_ok());
        let controller = get_controller(test_name, Some(12));
        assert!(controller.did_send_already_in_this_period());

        cleanup(controller, test_name);
    }

    #[test]
    fn test_did_send_already_in_this_period_outside_12h() {
        let test_name = "test_did_send_already_in_this_period_outside_12h";
        let initial_controller = get_controller(test_name, Some(12));

        let timestamp = Utc::now() - Duration::hours(13);
        let controller_data = ControllerData::new(timestamp);
        assert!(controller_data
            .save(initial_controller.db(), CollectionNames::CONTROLLER_DATA)
            .is_ok());

        // init controller again, to read self.last_time_data_sent from db
        assert!(initial_controller.dispose().is_ok());
        let controller = get_controller(test_name, Some(12));
        assert!(!controller.did_send_already_in_this_period());

        cleanup(controller, test_name);
    }

    #[test]
    fn test_did_send_already_in_this_period_inside_6h() {
        let test_name = "test_did_send_already_in_this_period_inside_6h";
        let initial_controller = get_controller(test_name, Some(6));

        let timestamp = Utc::now();
        let controller_data = ControllerData::new(timestamp);
        assert!(controller_data
            .save(initial_controller.db(), CollectionNames::CONTROLLER_DATA)
            .is_ok());

        // init controller again, to read self.last_time_data_sent from db
        assert!(initial_controller.dispose().is_ok());
        let controller = get_controller(test_name, Some(6));
        assert!(controller.did_send_already_in_this_period());

        cleanup(controller, test_name);
    }

    #[test]
    fn test_did_send_already_in_this_period_outside_6h() {
        let test_name = "test_did_send_already_in_this_period_outside_6h";
        let initial_controller = get_controller(test_name, Some(6));

        let timestamp = Utc::now() - Duration::hours(7);
        let controller_data = ControllerData::new(timestamp);
        assert!(controller_data
            .save(initial_controller.db(), CollectionNames::CONTROLLER_DATA)
            .is_ok());

        // init controller again, to read self.last_time_data_sent from db
        assert!(initial_controller.dispose().is_ok());
        let controller = get_controller(test_name, Some(6));
        assert!(!controller.did_send_already_in_this_period());

        cleanup(controller, test_name);
    }

    #[test]
    fn test_did_send_already_in_this_period_outside_twice_6h() {
        let test_name = "test_did_send_already_in_this_period_outside_twice_6h";
        let initial_controller = get_controller(test_name, Some(6));

        let timestamp = Utc::now() - Duration::hours(13);
        let controller_data = ControllerData::new(timestamp);
        assert!(controller_data
            .save(initial_controller.db(), CollectionNames::CONTROLLER_DATA)
            .is_ok());

        // init controller again, to read self.last_time_data_sent from db
        assert!(initial_controller.dispose().is_ok());
        let controller = get_controller(test_name, Some(6));
        assert!(!controller.did_send_already_in_this_period());

        cleanup(controller, test_name);
    }

    #[test]
    fn test_did_send_already_in_this_period_outside_thrice_6h() {
        let test_name = "test_did_send_already_in_this_period_outside_thrice_6h";
        let initial_controller = get_controller(test_name, Some(6));

        let timestamp = Utc::now() - Duration::hours(19);
        let controller_data = ControllerData::new(timestamp);
        assert!(controller_data
            .save(initial_controller.db(), CollectionNames::CONTROLLER_DATA)
            .is_ok());

        // init controller again, to read self.last_time_data_sent from db
        assert!(initial_controller.dispose().is_ok());
        let controller = get_controller(test_name, Some(6));
        assert!(!controller.did_send_already_in_this_period());

        cleanup(controller, test_name);
    }
}
