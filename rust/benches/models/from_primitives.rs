use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use paste::paste;

use xaynet_core::mask::{FromPrimitives, Model};

fn make_vector(bytes_size: usize) -> Vec<i32> {
    // 1 i32 -> 4 bytes
    assert_eq!(bytes_size % 4, 0);
    let n_elements = bytes_size / 4;
    vec![0_i32; n_elements]
}

macro_rules! fn_from_primitives {
    ($name: ident, $size: expr) => {
        paste! {
            #[allow(non_snake_case)]
            fn [<from_primitives $name>](crit: &mut Criterion) {
                let vector = make_vector($size);
                let name = &stringify!($name)[1..];

                let iter = vector.into_iter();
                crit.bench_function(
                    format!("convert {} model from primitive vector", name).as_str(),
                    |bench| {
                        bench.iter(|| Model::from_primitives(black_box(iter.clone())))
                    },
                );
            }
        }
    };
}

// 4 bytes
fn_from_primitives!(_tiny, 4);

// 100kB = 102_400 bytes
fn_from_primitives!(_100kB, 102_400);

// 1MB = 1_024_000 bytes
fn_from_primitives!(_1MB, 1_024_000);

criterion_group!(
    name = bench_model_from_primitives;
    config = Criterion::default().sample_size(1000).measurement_time(Duration::new(10, 0));
    targets =
        from_primitives_tiny,
        from_primitives_100kB,
        from_primitives_1MB,
);
criterion_main!(bench_model_from_primitives);
