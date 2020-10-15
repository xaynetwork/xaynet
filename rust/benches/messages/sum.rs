use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use xaynet_core::{
    crypto::{ByteObject, SecretSigningKey},
    message::Message,
    testutils::messages as helpers,
};

// `Message::to_bytes` takes a secret key as argument. It is not
// actually used, since the message we generate already contains a
// (dummy) signature.
fn participant_sk() -> SecretSigningKey {
    SecretSigningKey::from_slice(vec![2; 64].as_slice()).unwrap()
}

pub fn emit_sum(c: &mut Criterion) {
    let (sum_message, _) = helpers::message(helpers::sum::payload);
    let buf_len = sum_message.buffer_length();
    let mut pre_allocated_buf = vec![0; buf_len];

    // the benchmarks run under 20 ns. The results for such
    // benchmarks can vary a bit more so we:
    //   - eliminate outliers a bit more aggressively (confidence level)
    //   - increase the noise threshold
    //
    // Note: criterion always reports p = 0.0 so lowering the
    // significance level doesn't change anything
    let mut bench = c.benchmark_group("emit_sum");
    bench.confidence_level(0.9).noise_threshold(0.05);

    bench.bench_function("compute buffer length", |b| {
        b.iter(|| black_box(&sum_message).buffer_length())
    });

    bench.bench_function("emit sum message", |b| {
        b.iter(|| {
            sum_message.to_bytes(
                black_box(&mut pre_allocated_buf),
                black_box(&participant_sk()),
            )
        })
    });
}

pub fn parse_sum(c: &mut Criterion) {
    let sum_message = helpers::message(helpers::sum::payload).0;
    let mut bytes = vec![0; sum_message.buffer_length()];
    sum_message.to_bytes(&mut bytes, &participant_sk());

    // This benchmark is also quite unstable so make it a bit more
    // relaxed
    let mut bench = c.benchmark_group("parse_sum");
    bench.confidence_level(0.9).noise_threshold(0.05);
    bench.bench_function("parse from slice", |b| {
        b.iter(|| Message::from_byte_slice(&black_box(bytes.as_slice())))
    });
}

criterion_group!(name = benches;
                 // By default criterion collection 100 sample and the
                 // measurement time is 5 seconds, but the results are
                 // quite unstable with this configuration. This
                 // config makes the benchmarks running longer but
                 // provide more reliable results
                 config = Criterion::default().sample_size(1000).measurement_time(Duration::new(10, 0));
                 targets = emit_sum, parse_sum);
criterion_main!(benches);
