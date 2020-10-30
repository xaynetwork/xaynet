mod messages;
mod models;

use criterion::criterion_main;

criterion_main!(
    messages::messages::messages,
    models::parse_models::from_primitives,
    models::serialize_models::to_primitives,
);
