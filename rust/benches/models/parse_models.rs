use std::time::Duration;

use criterion::{black_box, criterion_group, Criterion};

use crate::models::utils;
use xaynet_core::mask::{FromPrimitives, Model};

pub fn parse_4bytes_model(c: &mut Criterion) {
    let vector_4bytes = utils::make_vector_4bytes();
    c.bench_function("parse model from 4 bytes vector", |bench| {
        bench.iter(|| Model::from_primitives(black_box(&mut vector_4bytes.clone().into_iter())))
    });
}

pub fn parse_100kB_model(c: &mut Criterion) {
    let vector_100kB = utils::make_vector_100kB();
    c.bench_function("parse model (bounded) from 100kB vector", |bench| {
        bench.iter(|| {
            Model::from_primitives_bounded(black_box(&mut vector_100kB.clone().into_iter()))
        })
    });
}

pub fn parse_1MB_model(c: &mut Criterion) {
    let vector_1MB = utils::make_vector_1MB();
    c.bench_function("parse model from 1MB vector", |bench| {
        bench.iter(|| Model::from_primitives(black_box(&mut vector_1MB.clone().into_iter())))
    });
}

criterion_group!(
    name = from_primitives;
    config = Criterion::default()
        .sample_size(1000)
        .measurement_time(Duration::new(10, 0));
    targets =
        parse_4bytes_model,
        parse_100kB_model,
        parse_1MB_model,
);
