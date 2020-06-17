#[macro_use]
extern crate tracing;

use std::sync::Arc;

use rayon::ThreadPoolBuilder;
use tower::{Service, ServiceBuilder};
use tracing_subscriber::*;

use xain_fl::{
    coordinator::{Coordinator, CoordinatorConfig, RoundSeed},
    crypto::{ByteObject, KeyPair},
    mask::{BoundType, DataType, GroupType, MaskConfig, ModelType},
    participant::{Participant, Task},
    services::{
        message_parser::MessageParserService,
        pre_processor::PreProcessorService,
        state_machine::StateMachineService,
        CoordinatorService,
    },
};

#[tokio::main]
async fn main() {
    let _fmt_subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

    sodiumoxide::init().unwrap();
    let config = CoordinatorConfig {
        mask_config: MaskConfig {
            group_type: GroupType::Prime,
            data_type: DataType::F32,
            bound_type: BoundType::B0,
            model_type: ModelType::M3,
        },
        initial_keys: KeyPair::generate(),
        initial_seed: RoundSeed::generate(),
        min_sum: 1,
        min_update: 1,
        // just so that we're sure our participant is a summer
        sum: 1.0,
        update: 0.00001,
    };

    let (coordinator, event_subscriber) = Coordinator::new(config).unwrap();

    let thread_pool = Arc::new(ThreadPoolBuilder::new().build().unwrap());

    let message_parser = ServiceBuilder::new()
        .buffer(10)
        .concurrency_limit(10)
        .service(MessageParserService::new(
            &event_subscriber,
            thread_pool.clone(),
        ));

    let pre_processor = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .service(PreProcessorService::new(&event_subscriber));

    let state_machine = ServiceBuilder::new()
        .buffer(100)
        .concurrency_limit(100)
        .service(StateMachineService::new(coordinator));

    let mut coordinator_svc = CoordinatorService::new(message_parser, pre_processor, state_machine);

    // pretend we are a participant that retrieves
    let mut participant = Participant::new().unwrap();

    // retrieve the round parameters using the subscriber. In practice,
    // we should have a service that services these parameters, but
    // for now the subscriber will do.
    let coordinator_pk = event_subscriber.keys_listener().get_latest().event.public;
    let round_params = event_subscriber.params_listener().get_latest().event;

    participant.compute_signatures(round_params.seed.as_slice());
    if participant.check_task(round_params.sum, round_params.update) != Task::Sum {
        panic!("not selected for sum task");
    }
    let sum_msg: Vec<u8> = participant.compose_sum_message(&coordinator_pk);
    futures::future::poll_fn(|cx| coordinator_svc.poll_ready(cx))
        .await
        .unwrap();
    let _ = coordinator_svc.call(sum_msg).await.unwrap();
}
