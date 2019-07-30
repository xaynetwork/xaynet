from typing import List

import numpy as np

from autofl.types import KerasWeights


# pylint: disable-msg=unused-argument
def weighted_avg(thetas: List[KerasWeights], coordinator) -> KerasWeights:
    # FIXME implement weighting
    return federated_averaging(thetas)


# FIXME align return type with KerasWeights type
# FIXME unify with weighted_federated_averaging
def federated_averaging(thetas: List[List[List[np.ndarray]]]) -> List[List[np.ndarray]]:
    theta_avg: List[List[np.ndarray]] = thetas[0]
    for theta in thetas[1:]:
        for layer_index, layer in enumerate(theta):
            for weight_index, weights in enumerate(layer):
                theta_avg[layer_index][weight_index] += weights
    for layer in theta_avg:
        for weights in layer:
            weights /= len(thetas)
    return theta_avg


# FIXME align return type with KerasWeights type
def weighted_federated_averaging(
    thetas: List[List[List[np.ndarray]]], weighting: np.ndarray
) -> List[List[np.ndarray]]:
    theta_avg: List[List[np.ndarray]] = thetas[0]
    for layer in theta_avg:
        for weights in layer:
            weights *= weighting[0]
    for theta, update_weighting in zip(thetas[1:], weighting[1:]):
        for layer_index, layer in enumerate(theta):
            for weight_index, weights in enumerate(layer):
                theta_avg[layer_index][weight_index] += update_weighting * weights
    weighting_sum = np.sum(weighting)
    for layer in theta_avg:
        for weights in layer:
            weights /= weighting_sum
    return theta_avg
