mod aggregation;

type ModelParams = ndarray::ArrayD<f64>;

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use ndarray::prelude::*;
    use ndarray_npy::ReadNpyExt;

    #[test]
    fn read_numpy_array() {
        let fixture = test_data_dir().join("arange(5.0).npy");
        let file = fs::File::open(fixture.as_path()).unwrap();

        let data = Array1::<f64>::read_npy(file).unwrap();
        assert_eq!(data, Array1::range(0.0, 5.0, 1.0));
    }

    fn test_data_dir() -> PathBuf {
        let mut res = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        res.push("test-data");
        res
    }
}
