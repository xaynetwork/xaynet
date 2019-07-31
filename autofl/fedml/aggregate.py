from typing import List

import numpy as np

from autofl.types import KerasWeights


# pylint: disable-msg=unused-argument
def weighted_avg(thetas: List[KerasWeights], coordinator) -> KerasWeights:
    # FIXME implement weighting
    return federated_averaging(thetas)


def federated_averaging(thetas: List[KerasWeights]) -> KerasWeights:
    weighting = np.ones((len(thetas),))
    return weighted_federated_averaging(thetas, weighting)


def weighted_federated_averaging(
    thetas: List[KerasWeights], weighting: np.ndarray
) -> KerasWeights:
    assert weighting.ndim == 1
    assert len(thetas) == weighting.shape[0]

    theta_avg: KerasWeights = thetas[0]
    for w in theta_avg:
        w *= weighting[0]

    # Aggregate (weighted) updates
    for theta, update_weighting in zip(thetas[1:], weighting[1:]):
        for w_index, w in enumerate(theta):
            theta_avg[w_index] += update_weighting * w

    weighting_sum = np.sum(weighting)
    for w in theta_avg:
        w /= weighting_sum

    return theta_avg
