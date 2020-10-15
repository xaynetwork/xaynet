#![allow(non_snake_case)]

mod utils;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use xaynet_core::message::{FromBytes, Update};

pub fn parse_tiny(c: &mut Criterion) {
    let (_, bytes) = utils::update::update_tiny();

    c.bench_function("parse tiny update from slice", |b| {
        b.iter(|| Update::from_byte_slice(&black_box(bytes.as_slice())))
    });

    // Note that an alternative way to write this benchmark is
    //
    // c.bench_function("parse tiny update from stream slower", |b| {
    //    b.iter(|| {
    //        Update::from_byte_stream(black_box(&mut bytes.as_slice().into_iter().cloned()))
    //    })
    // });
    //
    // However that is slightly slower so it seems that the method
    // that has the less overhead is to clone the iterator
    // directly. The cost of cloning should be negligible since we're
    // just cloning two pointers.
    let iter = bytes.clone().into_iter();
    c.bench_function("parse tiny update from stream", |b| {
        b.iter(|| Update::from_byte_stream(black_box(&mut iter.clone())))
    });
}

pub fn parse_100kB(c: &mut Criterion) {
    let (_, bytes) = utils::update::update_100kB();

    c.bench_function("parse 100kB update from slice", |b| {
        b.iter(|| Update::from_byte_slice(&black_box(bytes.as_slice())))
    });

    let iter = bytes.clone().into_iter();
    c.bench_function("parse 100k update from stream", |b| {
        b.iter(|| Update::from_byte_stream(black_box(&mut iter.clone())))
    });
}

pub fn parse_1MB(c: &mut Criterion) {
    let (_, bytes) = utils::update::update_1MB();

    c.bench_function("parse 1MB update from slice", |b| {
        b.iter(|| Update::from_byte_slice(&black_box(bytes.as_slice())))
    });

    let iter = bytes.clone().into_iter();
    c.bench_function("parse 1MB update from stream", |b| {
        b.iter(|| Update::from_byte_stream(black_box(&mut iter.clone())))
    });
}

pub fn parse_2MB(c: &mut Criterion) {
    let (_, bytes) = utils::update::update_2MB();

    c.bench_function("parse 2MB update from slice", |b| {
        b.iter(|| Update::from_byte_slice(&black_box(bytes.as_slice())))
    });

    let iter = bytes.clone().into_iter();
    c.bench_function("parse 2MB update from stream", |b| {
        b.iter(|| Update::from_byte_stream(black_box(&mut iter.clone())))
    });
}

pub fn parse_10MB(c: &mut Criterion) {
    let (_, bytes) = utils::update::update_10MB();

    c.bench_function("parse 10MB update from slice", |b| {
        b.iter(|| Update::from_byte_slice(&black_box(bytes.as_slice())))
    });

    let iter = bytes.clone().into_iter();
    c.bench_function("parse 10MB update from stream", |b| {
        b.iter(|| Update::from_byte_stream(black_box(&mut iter.clone())))
    });
}

criterion_group!(name = benches;
                 config = Criterion::default();
                 targets =
                    parse_tiny,
                    parse_100kB,
                    parse_1MB,
                    parse_2MB,
                    parse_10MB,
);
criterion_main!(benches);
