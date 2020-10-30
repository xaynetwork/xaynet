use xaynet_core::mask::{FromPrimitives, Model};

fn make_vector(bytes_size: usize) -> Vec<i32> {
    // 1 i32 -> 4 bytes
    let n_elements = bytes_size / 4;
    let vector = vec![0_i32; n_elements];
    vector
}

pub fn make_vector_4bytes() -> Vec<i32> {
    make_vector(4)
}

pub fn make_vector_100kB() -> Vec<i32> {
    // 100kB = 102400 bytes
    make_vector(102_400)
}

pub fn make_vector_1MB() -> Vec<i32> {
    // 1MB = 1048576 bytes
    make_vector(1_048_576)
}

pub fn make_model_4bytes() -> Model {
    let vector = make_vector_4bytes();
    Model::from_primitives_bounded(vector.into_iter())
}

pub fn make_model_100kB() -> Model {
    let vector = make_vector_100kB();
    Model::from_primitives_bounded(vector.into_iter())
}

pub fn make_model_1MB() -> Model {
    let vector = make_vector_1MB();
    Model::from_primitives_bounded(vector.into_iter())
}
