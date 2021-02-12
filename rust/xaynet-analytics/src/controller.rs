use anyhow::{Error, Result};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};

use crate::{
    data_combination::data_combiner::DataCombiner,
    database::{
        analytics_event::{data_model::AnalyticsEvent, repo::AnalyticsEventRepo},
        common::{IsarAdapter, Repo, SchemaGenerator},
        controller_data::{data_model::ControllerData, repo::ControllerDataRepo},
        isar::IsarDb,
        screen_route::{data_model::ScreenRoute, repo::ScreenRouteRepo},
    },
    sender::Sender,
};

struct AnalyticsController<'ctrl> {
    db: IsarDb,
    is_charging: bool,
    is_connected_to_wifi: bool,
    last_time_data_sent: Option<DateTime<Utc>>,
    analytics_event_repo: AnalyticsEventRepo<'ctrl>,
    controller_data_repo: ControllerDataRepo<'ctrl>,
    screen_route_repo: ScreenRouteRepo<'ctrl>,
    combiner: DataCombiner,
    sender: Sender,
    send_data_frequency: Duration,
}

// TODO: remove allow dead code when AnalyticsController is integrated with FFI layer: https://xainag.atlassian.net/browse/XN-1415
#[allow(dead_code)]
impl<'ctrl> AnalyticsController<'ctrl> {
    const ANALYTICS_EVENT_COLLECTION_NAME: &'ctrl str = "analytics_events";
    const CONTROLLER_DATA_COLLECTION_NAME: &'ctrl str = "controller_data";
    const SCREEN_ROUTE_COLLECTION_NAME: &'ctrl str = "screen_routes";
    const SEND_DATA_FREQUENCY_HOURS: i64 = 24;

    pub fn init(
        path: String,
        is_charging: bool,
        is_connected_to_wifi: bool,
    ) -> Result<Self, Error> {
        let analytics_event_repo = AnalyticsEventRepo::new(Self::ANALYTICS_EVENT_COLLECTION_NAME);
        let controller_data_repo = ControllerDataRepo::new(Self::CONTROLLER_DATA_COLLECTION_NAME);
        let screen_route_repo = ScreenRouteRepo::new(Self::SCREEN_ROUTE_COLLECTION_NAME);
        let schemas = vec![
            AnalyticsEvent::get_schema(Self::ANALYTICS_EVENT_COLLECTION_NAME)?,
            ControllerData::get_schema(Self::CONTROLLER_DATA_COLLECTION_NAME)?,
            ScreenRoute::get_schema(Self::SCREEN_ROUTE_COLLECTION_NAME)?,
        ];
        let db = IsarDb::new(&path, schemas)?;
        let last_time_data_sent = Self::get_last_time_data_sent(&controller_data_repo, &db)?;
        let combiner = DataCombiner;
        let sender = Sender;
        Ok(AnalyticsController {
            db,
            is_charging,
            is_connected_to_wifi,
            last_time_data_sent,
            analytics_event_repo,
            controller_data_repo,
            screen_route_repo,
            combiner,
            sender,
            send_data_frequency: Duration::hours(Self::SEND_DATA_FREQUENCY_HOURS),
        })
    }

    pub fn dispose(self) -> Result<(), Error> {
        self.db.dispose()
    }

    pub fn save_analytics_event(&self, event_bytes: &[u8]) -> Result<(), Error> {
        let event = AnalyticsEvent::read(event_bytes);
        self.check_for_new_screen_route(&event)?;
        self.analytics_event_repo.add(&event, &self.db)
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

    fn check_for_new_screen_route(&self, event: &AnalyticsEvent) -> Result<(), Error> {
        match event.screen_route {
            Some(screen_route) => self.add_screen_route_if_new(screen_route),
            None => Ok(()),
        }
    }

    fn add_screen_route_if_new(&self, screen_route: &ScreenRoute) -> Result<(), Error> {
        if !self
            .screen_route_repo
            .get_all(&self.db)?
            .contains(screen_route)
        {
            self.screen_route_repo.add(screen_route, &self.db)
        } else {
            Ok(())
        }
    }

    fn get_last_time_data_sent(
        repo: &ControllerDataRepo,
        db: &IsarDb,
    ) -> Result<Option<DateTime<Utc>>, Error> {
        match repo.get_all(db)?.last() {
            Some(data) => Ok(Some(data.time_data_sent)),
            None => Ok(None),
        }
    }

    fn should_send_data(&self) -> bool {
        self.can_send_data() && !self.did_send_already_in_this_period()
    }

    fn can_send_data(&self) -> bool {
        self.is_charging && self.is_connected_to_wifi
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
        let events = self.analytics_event_repo.get_all(&self.db)?;
        let screen_routes = self.screen_route_repo.get_all(&self.db)?;
        let time_data_sent = Utc::now();
        self.sender
            .send(self.combiner.init_data_points(&events, &screen_routes)?)
            .and_then(|_| {
                self.controller_data_repo
                    .add(&ControllerData::new(time_data_sent), &self.db)
            })
            .map(|_| self.last_time_data_sent = Some(time_data_sent))
    }
}
