use crate::{Model, Tensor};

#[allow(unused)]
pub(crate) fn federated_average(models: &[Model], weights: &[u64]) -> Model {
    assert!(!models.is_empty());
    assert_eq!(models.len(), weights.len());

    let mut res: Model = models[0].iter().map(|tensor| Tensor::default(tensor.dim())).collect();

    for (model, &weight) in models.iter().zip(weights) {
        for (res_t, model_t) in res.iter_mut().zip(model.iter()) {
            res_t.scaled_add(weight as f64, model_t);
        }
    }

    let total_weight = weights.iter().sum::<u64>() as f64;
    for tensor in res.iter_mut() {
        *tensor /= total_weight;
    }

    res
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn federated_average_smoke_test() {
        let weights = &[1, 2];

        let model1: Model = vec![array![3.0, 1.0, 0.0].into_dyn()];
        let model2: Model = vec![array![0.0, 1.0, 3.0].into_dyn()];

        let res = federated_average(&[model1, model2], weights);

        assert_eq!(res, vec![array![1.0, 1.0, 2.0].into_dyn()])
    }
}
