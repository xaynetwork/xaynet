#![allow(non_snake_case)]

mod utils;
pub use utils::*;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

use xaynet_core::mask::{FromPrimitives, Model};

pub fn parse_4bytes_model(c: &mut Criterion) {
    let vector_4bytes = utils::vector_or_model::make_vector_4bytes();
    c.bench_function("parse model from 4 bytes vector", |bench| {
        bench.iter(|| Model::from_primitives(black_box(&mut vector_4bytes.clone().into_iter())))
    });
}

pub fn parse_100kB_model(c: &mut Criterion) {
    let vector_100kB = utils::vector_or_model::make_vector_100kB();
    c.bench_function("parse model (bounded) from 100kB vector", |bench| {
        bench.iter(|| {
            Model::from_primitives_bounded(black_box(&mut vector_100kB.clone().into_iter()))
        })
    });
}

pub fn parse_1MB_model(c: &mut Criterion) {
    let vector_1MB = utils::vector_or_model::make_vector_1MB();
    c.bench_function("parse model from 1MB vector", |bench| {
        bench.iter(|| Model::from_primitives(black_box(&mut vector_1MB.clone().into_iter())))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(1000)
        .measurement_time(Duration::new(10, 0));
    targets =
        parse_4bytes_model,
        parse_100kB_model,
        parse_1MB_model,
);
criterion_main!(benches);
