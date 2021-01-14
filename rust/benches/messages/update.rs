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

macro_rules! fn_from_bytes {
    ($name: ident, $dict_len: expr, $mask_len: expr, $total_len: expr) => {
        paste! {
            #[allow(non_snake_case)]
            fn [<from_bytes $name>](crit: &mut Criterion) {
                let (_, bytes) = make_update($dict_len, $mask_len, $total_len);
                let name = &stringify!($name)[1..];
                let mut crit = crit.benchmark_group(format!("deserialize {} update from bytes", name));

                crit.bench_function(
                    format!("deserialize {} update from bytes slice", name).as_str(),
                    |bench| {
                        bench.iter(|| Update::from_byte_slice(&black_box(bytes.as_slice())))
                    },
                );

                // it's less overhead to clone the iterator of bytes instead of re-creating it
                // again in every benchmark iteration
                let iter = bytes.into_iter();
                crit.bench_function(
                    format!("deserialize {} update from bytes stream", name).as_str(),
                    |bench| {
                        bench.iter(|| Update::from_byte_stream(black_box(&mut iter.clone())))
                    },
                );
            }
        }
    };
}

// Get an update that corresponds to:
// - 1 sum participant (1 entry in the seed dict)
// - a 42 bytes serialized masked model
fn_from_bytes!(_tiny, 116, 42, 286);

// Get an update that corresponds to:
// - 1k sum participants (1k entries in the seed dict)
// - a 6kB serialized masked model
fn_from_bytes!(_100kB, 112_004, 6_018, 118_150);

// Get an update that corresponds to:
// - 10k sum participants (10k entries in the seed dict)
// - a 60kB serialized masked model
fn_from_bytes!(_1MB, 1_120_004, 60_018, 1_180_150);

// Get an update that corresponds to:
// - 10k sum participants (10k entries in the seed dict)
// - a ~1MB serialized masked model
fn_from_bytes!(_2MB, 1_120_004, 1_000_020, 2_120_152);

// Get an update that corresponds to:
// - 10k sum participants (10k entries in the seed dict)
// - a ~9MB serialized masked model
fn_from_bytes!(_10MB, 1_120_004, 9_000_018, 10_120_150);

criterion_group!(
    name = bench_update_message;
    config = Criterion::default();
    targets =
        from_bytes_tiny,
        from_bytes_100kB,
        from_bytes_1MB,
        from_bytes_2MB,
        from_bytes_10MB,
);
criterion_main!(bench_update_message);
