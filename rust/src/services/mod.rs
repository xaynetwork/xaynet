mod fetchers;
mod messages;

pub use self::{
    fetchers::{FetchError, Fetcher},
    messages::{PetMessageError, PetMessageHandler},
};

use crate::{
    services::{
        fetchers::{
            FetcherService,
            MaskLengthService,
            ModelService,
            RoundParamsService,
            ScalarService,
            SeedDictService,
            SumDictService,
        },
        messages::{
            MessageParserService,
            PetMessageService,
            PreProcessorService,
            StateMachineService,
        },
    },
    state_machine::{
        events::EventSubscriber,
        requests::{Request, RequestSender},
    },
    utils::trace::Traced,
};

use std::sync::Arc;

use rayon::ThreadPoolBuilder;
use tower::ServiceBuilder;

pub fn fetcher(event_subscriber: &EventSubscriber) -> impl Fetcher + Sync + Send + 'static {
    let round_params = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .service(RoundParamsService::new(event_subscriber));

    let mask_length = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .service(MaskLengthService::new(event_subscriber));

    let scalar = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .service(ScalarService::new(event_subscriber));

    let model = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .service(ModelService::new(event_subscriber));

    let sum_dict = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .service(SumDictService::new(event_subscriber));

    let seed_dict = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .service(SeedDictService::new(event_subscriber));

    FetcherService::new(
        round_params,
        sum_dict,
        seed_dict,
        mask_length,
        scalar,
        model,
    )
}

pub fn message_handler(
    event_subscriber: &EventSubscriber,
    requests_tx: RequestSender<Traced<Request>>,
) -> impl PetMessageHandler + Sync + Send + 'static {
    // TODO: make this configurable. Users should be able to
    // choose how many threads they want etc.
    //
    // TODO: don't unwrap
    let thread_pool = Arc::new(ThreadPoolBuilder::new().build().unwrap());

    let message_parser = ServiceBuilder::new()
        // allow processing 100 request concurrently, and allow up to
        // 10 requests to be pending. Once 100 requests are being
        // processed and 100 are queued, the service will report that
        // it's not ready.
        //
        // FIXME: what's a good concurrency limit? Should we limit
        // the number of concurrent messages being processed to
        // the number of threads in the rayon thread-pool? Or is
        // rayon smart enough to avoid too many context switches?
        .buffer(100)
        .concurrency_limit(100)
        .service(MessageParserService::new(event_subscriber, thread_pool));

    let pre_processor = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .service(PreProcessorService::new(event_subscriber));

    let state_machine = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .service(StateMachineService::new(requests_tx));

    PetMessageService::new(message_parser, pre_processor, state_machine)
}
