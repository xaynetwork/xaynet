#![allow(non_snake_case)]

mod utils;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

use xaynet_core::mask::{IntoPrimitives, Model};

fn serialize(model: &Model) -> Vec<i32> {
    model
        .clone()
        .into_primitives_unchecked()
        .collect::<Vec<i32>>()
}

pub fn serialize_4bytes_model(c: &mut Criterion) {
    let model_4bytes = utils::vector_or_model::make_model_4bytes();
    c.bench_function("serialize 4 bytes model into primitives", |bench| {
        bench.iter(|| serialize(black_box(&model_4bytes)))
    });
}

pub fn serialize_100kB_model(c: &mut Criterion) {
    let model_100kB = utils::vector_or_model::make_model_100kB();
    c.bench_function("serialize 100kB model into primitives", |bench| {
        bench.iter(|| serialize(black_box(&model_100kB)))
    });
}

pub fn serialize_1MB_model(c: &mut Criterion) {
    let model_1MB = utils::vector_or_model::make_model_1MB();
    c.bench_function("serialize 1MB model into primitives", |bench| {
        bench.iter(|| serialize(black_box(&model_1MB)))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(1000)
        .measurement_time(Duration::new(10, 0));
    targets =
        serialize_4bytes_model,
        serialize_100kB_model,
        serialize_1MB_model,
);
criterion_main!(benches);
