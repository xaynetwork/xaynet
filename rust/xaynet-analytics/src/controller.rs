use anyhow::{Error, Result};
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
    const SEND_DATA_FREQUENCY_HOURS: i64 = 24;

    pub fn init(
        path: String,
        is_charging: bool,
        is_connected_to_wifi: bool,
    ) -> Result<Self, Error> {
        let schemas = vec![
            AnalyticsEventAdapter::get_schema(&CollectionNames::ANALYTICS_EVENTS)?,
            ControllerDataAdapter::get_schema(&CollectionNames::CONTROLLER_DATA)?,
            ScreenRouteAdapter::get_schema(&CollectionNames::SCREEN_ROUTES)?,
        ];
        let db = IsarDb::new(&path, schemas)?;
        let last_time_data_sent = Self::get_last_time_data_sent(&db)?;
        Ok(AnalyticsController {
            db,
            is_charging,
            is_connected_to_wifi,
            last_time_data_sent,
            combiner: DataCombiner,
            sender: Sender,
            send_data_frequency: Duration::hours(Self::SEND_DATA_FREQUENCY_HOURS),
        })
    }

    pub fn dispose(self) -> Result<(), Error> {
        self.db.dispose()
    }

    pub fn save_analytics_event(
        &self,
        name: &str,
        event_type: AnalyticsEventType,
        option_screen_route_name: Option<&str>,
    ) -> Result<(), Error> {
        let option_screen_route = option_screen_route_name
            .map(|screen_route_name| self.add_screen_route_if_new(screen_route_name))
            .transpose()?;

        let event = AnalyticsEvent::new(
            name.to_string(),
            event_type,
            Utc::now(),
            option_screen_route,
        );
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
        let can_send_data = self.is_charging && self.is_connected_to_wifi;
        let should_send_data = can_send_data && !self.did_send_already_in_this_period();
        if should_send_data {
            self.send_data()
        } else {
            Ok(())
        }
    }

    fn add_screen_route_if_new(&self, screen_route_name: &str) -> Result<ScreenRoute, Error> {
        let existing_screen_routes =
            ScreenRoute::get_all(&self.db, &CollectionNames::SCREEN_ROUTES)?;
        if let Some(existing_screen_route) = existing_screen_routes
            .into_iter()
            .find(|existing_route| existing_route.name == screen_route_name)
        {
            Ok(existing_screen_route)
        } else {
            let screen_route = ScreenRoute::new(screen_route_name, Utc::now());
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

    // TODO: review and debug this method during https://xainag.atlassian.net/browse/XN-1560
    fn did_send_already_in_this_period(&self) -> bool {
        self.last_time_data_sent.is_some() && {
            let tomorrow = Utc::now() + Duration::days(1);
            let midnight_after_current_time: DateTime<Utc> = DateTime::from_utc(
                NaiveDate::from_ymd(tomorrow.year(), tomorrow.month(), tomorrow.day())
                    .and_hms(0, 0, 0),
                Utc,
            );
            let start_of_current_period = midnight_after_current_time - self.send_data_frequency;
            self.last_time_data_sent.unwrap() < start_of_current_period
        }
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
