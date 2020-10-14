use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use xaynet_core::{
    crypto::{ByteObject, PublicEncryptKey, PublicSigningKey, SecretSigningKey, Signature},
    message::{Message, Payload, Sum, Tag},
};

mod message {
    use super::*;

    fn signature() -> Signature {
        Signature::from_slice(vec![1; 64].as_slice()).unwrap()
    }

    fn participant_pk() -> PublicSigningKey {
        PublicSigningKey::from_slice(vec![2; 32].as_slice()).unwrap()
    }

    fn coordinator_pk() -> PublicEncryptKey {
        PublicEncryptKey::from_slice(vec![3; 32].as_slice()).unwrap()
    }

    pub fn message(payload: Payload) -> Message {
        let tag = match payload {
            Payload::Sum(_) => Tag::Sum,
            Payload::Update(_) => Tag::Update,
            Payload::Sum2(_) => Tag::Sum2,
            _ => unimplemented!(),
        };
        Message {
            // For `to_bytes` benches, it's important that the
            // signature is already set, otherwise, it would be
            // computed which would affect the benchmark
            signature: Some(signature()),
            participant_pk: participant_pk(),
            coordinator_pk: coordinator_pk(),
            payload,
            is_multipart: false,
            tag,
        }
    }

    pub fn participant_sk() -> SecretSigningKey {
        SecretSigningKey::from_slice(vec![255; 64].as_slice()).unwrap()
    }
}

mod sum {
    use super::*;

    pub fn payload() -> Sum {
        let sum_signature = Signature::from_slice(vec![4; 64].as_slice()).unwrap();

        let ephm_pk = PublicEncryptKey::from_slice(vec![5; 32].as_slice()).unwrap();
        Sum {
            sum_signature,
            ephm_pk,
        }
    }

    pub fn message() -> Message {
        message::message(payload().into())
    }
}

pub fn emit_sum(c: &mut Criterion) {
    let sum_message = sum::message();
    let sk = message::participant_sk();
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
        b.iter(|| sum_message.to_bytes(black_box(&mut pre_allocated_buf), black_box(&sk)))
    });
}

pub fn parse_sum(c: &mut Criterion) {
    let sum_message = sum::message();
    let mut bytes = vec![0; sum_message.buffer_length()];
    sum_message.to_bytes(&mut bytes, &message::participant_sk());

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
