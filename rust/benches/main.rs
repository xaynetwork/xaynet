mod messages;
mod models;

use criterion::criterion_main;

criterion_main!(
    messages::sum::sum,
    models::parse_models::from_primitives,
    models::serialize_models::to_primitives,
);
