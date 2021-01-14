use std::{iter, time::Duration};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use num::{bigint::BigInt, rational::Ratio};
use paste::paste;

use xaynet_core::mask::{IntoPrimitives, Model};

fn make_model(bytes_size: usize) -> Model {
    // 1 i32 -> 4 bytes
    assert_eq!(bytes_size % 4, 0);
    let n_elements = bytes_size / 4;
    iter::repeat(Ratio::from(BigInt::from(0)))
        .take(n_elements)
        .collect()
}

macro_rules! fn_to_primitives {
    ($name: ident, $size: expr) => {
        paste! {
            #[allow(non_snake_case)]
            fn [<to_primitives $name>](crit: &mut Criterion) {
                let model = make_model($size);
                let name = &stringify!($name)[1..];

                crit.bench_function(
                    format!("convert {} model to primitive vector", name).as_str(),
                    |bench| {
                        bench.iter(|| black_box(&model).to_primitives().collect::<Result<Vec<i32>, _>>())
                    }
                );
            }
        }
    };
}

// 4 bytes
fn_to_primitives!(_tiny, 4);

// 100kB = 102_400 bytes
fn_to_primitives!(_100kB, 102_400);

// 1MB = 1_024_000 bytes
fn_to_primitives!(_1MB, 1_024_000);

criterion_group!(
    name = bench_model_to_primitives;
    config = Criterion::default().sample_size(1000).measurement_time(Duration::new(10, 0));
    targets =
        to_primitives_tiny,
        to_primitives_100kB,
        to_primitives_1MB,
);
criterion_main!(bench_model_to_primitives);
