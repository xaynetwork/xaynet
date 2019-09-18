use ndarray::prelude::*;

use crate::ModelParams;

#[allow(unused)]
pub(crate) fn federated_average(thetas: &[(ModelParams)], weights: &[u64]) -> ModelParams {
    assert!(!thetas.is_empty());
    assert_eq!(thetas.len(), weights.len());
    let mut res = ModelParams::default(thetas[0].dim());
    for (theta, &weight) in thetas.iter().zip(weights) {
        res.scaled_add(weight as f64, theta);
    }
    let total_weight = weights.iter().sum::<u64>() as f64;
    res /= total_weight;
    res
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn federated_average_smoke_test() {
        let weights = &[1, 2];

        let theta1: ModelParams = array![3.0, 1.0, 0.0].into_dyn();
        let theta2: ModelParams = array![0.0, 1.0, 3.0].into_dyn();

        let res = federated_average(&[theta1, theta2], weights);

        assert_eq!(res, array![1.0, 1.0, 2.0].into_dyn())
    }
}
