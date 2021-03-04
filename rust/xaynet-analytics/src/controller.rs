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

struct AnalyticsController {
    db: IsarDb,
    is_charging: bool,
    is_connected_to_wifi: bool,
    last_time_data_sent: Option<DateTime<Utc>>,
    combiner: DataCombiner,
    sender: Sender,
    send_data_frequency: Duration,
}

// TODO: remove allow dead code when AnalyticsController is integrated with FFI layer: https://xainag.atlassian.net/browse/XN-1415
#[allow(dead_code)]
impl AnalyticsController {
    const MAX_SEND_DATA_FREQUENCY_HOURS: u8 = 24;

    pub fn init(
        path: String,
        is_charging: bool,
        is_connected_to_wifi: bool,
        input_send_data_frequency: Option<u8>,
    ) -> Result<Self, Error> {
        let schemas = vec![
            AnalyticsEventAdapter::get_schema(&CollectionNames::ANALYTICS_EVENTS)?,
            ControllerDataAdapter::get_schema(&CollectionNames::CONTROLLER_DATA)?,
            ScreenRouteAdapter::get_schema(&CollectionNames::SCREEN_ROUTES)?,
        ];
        let db = IsarDb::new(&path, schemas)?;
        let last_time_data_sent = Self::get_last_time_data_sent(&db)?;
        let send_data_frequency = Self::validate_send_data_frequency(input_send_data_frequency)?;

        Ok(AnalyticsController {
            db,
            is_charging,
            is_connected_to_wifi,
            last_time_data_sent,
            combiner: DataCombiner,
            sender: Sender,
            send_data_frequency,
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

    fn validate_send_data_frequency(
        input_send_data_frequency: Option<u8>,
    ) -> Result<Duration, Error> {
        let send_data_frequency =
            input_send_data_frequency.unwrap_or(Self::MAX_SEND_DATA_FREQUENCY_HOURS);
        if send_data_frequency > Self::MAX_SEND_DATA_FREQUENCY_HOURS {
            Err(anyhow!(
                "input_send_data_frequency must be between 0 and {}",
                Self::MAX_SEND_DATA_FREQUENCY_HOURS
            ))
        } else {
            Ok(Duration::hours(send_data_frequency as i64))
        }
    }

    fn should_send_data(&self) -> bool {
        let can_send_data = self.is_charging && self.is_connected_to_wifi;
        can_send_data && !self.did_send_already_in_this_period()
    }

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

    /// This method implements a sliding 'time window' of self.send_data_frequency duration, to check whether we have
    /// already sent data in the current window, or not.
    ///
    /// An alternative implementation could be based on simply checking whether:
    /// last_time_data_sent > Utc::now() - self.send_data_frequency
    ///
    /// In the current implementation, it might be easier to then group the aggregated data on the coordinator side,
    /// to then be displayed in the UI, especially if self.send_data_frequency = Duration::hours(24).
    ///
    /// The more dynamic approach however implies that if, for example, self.send_data_frequency = Duration::hours(6),
    /// and the last time we sent the data was at 5AM, we would be able to send again at 7AM, while with the simpler solution
    /// we wouldn't be able to send again until 11AM.
    ///
    /// The correct approach to be chosen should very much depend on the amount of data available for aggregation,
    /// and it's possible that Self::MAX_SEND_DATA_FREQUENCY_HOURS should be increased to more than 24.
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
                let mut end_of_current_period = start_of_day + self.send_data_frequency;
                while now > end_of_current_period {
                    end_of_current_period = end_of_current_period + self.send_data_frequency;
                }
                let start_of_current_period = end_of_current_period - self.send_data_frequency;
                last_time_data_sent > start_of_current_period
            })
            .unwrap_or(false)
    }

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
        assert!(controller
            .save_analytics_event(name, event_type, timestamp, None)
            .is_ok());

        let analytics_event = AnalyticsEvent::new(name, event_type, timestamp, None);
        let all_analytics_events =
            AnalyticsEvent::get_all(controller.db(), CollectionNames::ANALYTICS_EVENTS);
        assert_eq!(
            all_analytics_events.unwrap().first(),
            Some(&analytics_event)
        );

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
            AnalyticsEvent::get_all(controller.db(), CollectionNames::ANALYTICS_EVENTS);
        assert_eq!(
            all_analytics_events.unwrap().first(),
            Some(&analytics_event)
        );

        cleanup(controller, test_name);
    }

    #[test]
    fn test_change_connectivity_status() {
        let test_name = "test_change_connectivity_status";
        let mut controller = get_controller(test_name, None);
        assert_eq!(controller.is_connected_to_wifi, true);

        controller.change_connectivity_status();
        assert_eq!(controller.is_connected_to_wifi, false);

        cleanup(controller, test_name);
    }

    #[test]
    fn test_change_state_of_charge() {
        let test_name = "test_change_state_of_charge";
        let mut controller = get_controller(test_name, None);
        assert_eq!(controller.is_charging, true);

        controller.change_state_of_charge();
        assert_eq!(controller.is_charging, false);

        cleanup(controller, test_name);
    }

    #[test]
    fn test_validate_send_data_frequency_when_none() {
        assert_eq!(
            AnalyticsController::validate_send_data_frequency(None).unwrap(),
            Duration::hours(AnalyticsController::MAX_SEND_DATA_FREQUENCY_HOURS as i64)
        )
    }

    #[test]
    fn test_validate_send_data_frequency_when_more_than_24() {
        assert!(AnalyticsController::validate_send_data_frequency(Some(25)).is_err());
    }

    #[test]
    fn test_validate_send_data_frequency_when_less_than_24() {
        assert_eq!(
            AnalyticsController::validate_send_data_frequency(Some(6)).unwrap(),
            Duration::hours(6)
        )
    }

    #[test]
    fn test_validate_send_data_frequency_when_0() {
        assert_eq!(
            AnalyticsController::validate_send_data_frequency(Some(0)).unwrap(),
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
        assert_eq!(
            controller
                .add_screen_route_if_new(screen_route_name, timestamp)
                .unwrap(),
            screen_route
        );
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

        assert_eq!(controller.did_send_already_in_this_period(), false);

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
        assert_eq!(controller.did_send_already_in_this_period(), true);

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
        assert_eq!(controller.did_send_already_in_this_period(), false);

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
        assert_eq!(controller.did_send_already_in_this_period(), true);

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
        assert_eq!(controller.did_send_already_in_this_period(), false);

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
        assert_eq!(controller.did_send_already_in_this_period(), true);

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
        assert_eq!(controller.did_send_already_in_this_period(), false);

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
        assert_eq!(controller.did_send_already_in_this_period(), false);

        cleanup(controller, test_name);
    }

    #[test]
    fn test_did_send_already_in_this_period_outside_trice_6h() {
        let test_name = "test_did_send_already_in_this_period_outside_trice_6h";
        let initial_controller = get_controller(test_name, Some(6));

        let timestamp = Utc::now() - Duration::hours(19);
        let controller_data = ControllerData::new(timestamp);
        assert!(controller_data
            .save(initial_controller.db(), CollectionNames::CONTROLLER_DATA)
            .is_ok());

        // init controller again, to read self.last_time_data_sent from db
        assert!(initial_controller.dispose().is_ok());
        let controller = get_controller(test_name, Some(6));
        assert_eq!(controller.did_send_already_in_this_period(), false);

        cleanup(controller, test_name);
    }
}
