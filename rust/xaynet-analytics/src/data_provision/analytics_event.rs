use chrono::{DateTime, Utc};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AnalyticsEventType {
    AppEvent,
    Error,
    ScreenEnter,
    UserAction,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AnalyticsEvent {
    pub name: String,
    pub event_type: AnalyticsEventType,
    pub timestamp: DateTime<Utc>,
    pub screen_route: Option<String>,
}

impl AnalyticsEvent {
    pub fn new(
        name: String,
        event_type: AnalyticsEventType,
        screen_route: Option<String>,
    ) -> AnalyticsEvent {
        AnalyticsEvent {
            name,
            event_type,
            screen_route,
            timestamp: chrono::offset::Utc::now(),
        }
    }
}
