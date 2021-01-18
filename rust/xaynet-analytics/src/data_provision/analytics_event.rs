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
    pub fn new<N: Into<String>, R: Into<Option<String>>>(
        name: N,
        event_type: AnalyticsEventType,
        timestamp: DateTime<Utc>,
        screen_route: R,
    ) -> AnalyticsEvent {
        AnalyticsEvent {
            name: name.into(),
            event_type,
            timestamp,
            screen_route: screen_route.into(),
        }
    }
}
