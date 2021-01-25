use futures::Future;

use crate::{
    app::drain::Watch,
    rest::{serve, RestError},
    services,
    settings::ApiSettings,
    state_machine::{
        events::EventSubscriber,
        requests::{RequestSender, UserRequestSender},
    },
};

pub fn init(
    api_settings: ApiSettings,
    event_subscriber: EventSubscriber,
    requests_tx: RequestSender,
    user_requests_tx: UserRequestSender,
    shutdown: Watch,
) -> impl Future<Output = Result<(), RestError>> + 'static {
    tracing::debug!("initialize");
    let fetcher = services::fetchers::fetcher(&event_subscriber);
    let message_handler =
        services::messages::PetMessageHandler::new(&event_subscriber, requests_tx);

    serve(
        api_settings,
        fetcher,
        message_handler,
        shutdown,
        user_requests_tx,
    )
}
