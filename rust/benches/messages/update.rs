use criterion::{black_box, criterion_group, criterion_main, Criterion};
use paste::paste;

use xaynet_core::{
    message::{FromBytes, ToBytes, Update},
    testutils::multipart as helpers,
};

fn make_update(dict_len: usize, mask_len: usize, total_expected_len: usize) -> (Update, Vec<u8>) {
    let update = helpers::update(dict_len, mask_len);
    // just check that we made our calculation right
    // message size = dict_len + mask_len + 64*2
    assert_eq!(update.buffer_length(), total_expected_len);
    let mut bytes = vec![0; update.buffer_length()];
    update.to_bytes(&mut bytes);
    (update, bytes)
}

macro_rules! fn_parse {
    ($name: ident, $dict_len: expr, $mask_len: expr, $total_len: expr) => {
        paste! {
            #[allow(non_snake_case)]
            pub fn [<parse $name>](c: &mut Criterion) {
                let (_, bytes) = make_update($dict_len, $mask_len, $total_len);
                let size = &stringify!($name)[1..];

                c.bench_function(format!("parse {} update from slice", size).as_str(), |b| {
                    b.iter(|| Update::from_byte_slice(&black_box(bytes.as_slice())))
                });

                // it's less overhead to clone the iterator of bytes instead of re-creating it
                // again in every benchmark iteration
                let iter = bytes.into_iter();
                c.bench_function(format!("parse {} update from stream", size).as_str(), |b| {
                    b.iter(|| Update::from_byte_stream(black_box(&mut iter.clone())))
                });
            }
        }
    };
}

// Get an update that corresponds to:
// - 1 sum participant (1 entry in the seed dict)
// - a 42 bytes serialized masked model
fn_parse!(_tiny, 116, 42, 286);

// Get an update that corresponds to:
// - 1k sum participants (1k entries in the seed dict)
// - a 6kB serialized masked model
fn_parse!(_100kB, 112_004, 6_018, 118_150);

// Get an update that corresponds to:
// - 10k sum participants (10k entries in the seed dict)
// - a 60kB serialized masked model
fn_parse!(_1MB, 1_120_004, 60_018, 1_180_150);

// Get an update that corresponds to:
// - 10k sum participants (10k entries in the seed dict)
// - a ~1MB serialized masked model
fn_parse!(_2MB, 1_120_004, 1_000_020, 2_120_152);

// Get an update that corresponds to:
// - 10k sum participants (10k entries in the seed dict)
// - a ~9MB serialized masked model
fn_parse!(_10MB, 1_120_004, 9_000_018, 10_120_150);

criterion_group!(
    name = bench_update_message;
    config = Criterion::default();
    targets =
        parse_tiny,
        parse_100kB,
        parse_1MB,
        parse_2MB,
        parse_10MB,
);
criterion_main!(bench_update_message);
