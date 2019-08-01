from abc import ABC
from typing import List, Tuple

import numpy as np
from absl import logging

from autofl.types import KerasWeights

from .evaluator import Evaluator


class Aggregator(ABC):
    def __init__(self):
        pass

    def aggregate(self, thetas: List[KerasWeights]) -> KerasWeights:
        raise NotImplementedError()


class WeightedAverageAgg(Aggregator):
    def aggregate(self, thetas: List[KerasWeights]) -> KerasWeights:
        return weighted_agg(thetas)


class EvoAgg(Aggregator):
    def __init__(self, evaluator: Evaluator):
        super().__init__()
        self.evaluator = evaluator

    def aggregate(self, thetas: List[KerasWeights]) -> KerasWeights:
        return evo_agg(thetas, self.evaluator, False)


def weighted_agg(thetas: List[KerasWeights],) -> KerasWeights:
    # FIXME implement #examples-based weighting
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


def evo_agg(
    thetas: List[KerasWeights], evaluator: Evaluator, verbose=False
) -> KerasWeights:
    """
    - Init different weightings
    - Aggregate thetas according to those weightings ("candidates")
    - Evaluate all candidates on the validation set
    - Pick (a) best candidate, or (b) average of n best candidates
    """
    # Compute candidates
    # TODO in parallel, do:
    theta_prime_candidates = []
    for i in range(3):
        candidate = compute_candidate(thetas, evaluator)
        if verbose:
            logging.info(
                "candidate {} (weighting {}): {} loss".format(
                    i, candidate[0], candidate[2]
                )
            )
        theta_prime_candidates.append(candidate)
    # Return best candidate
    best_candidate = pick_best_candidate(theta_prime_candidates)
    return best_candidate


def pick_best_candidate(candidates: List) -> KerasWeights:
    _, best_candidate, best_loss, _ = candidates[0]
    for _, candidate, loss, _ in candidates[1:]:
        if loss < best_loss:
            best_candidate = candidate
            best_loss = loss
    return best_candidate


def compute_candidate(
    thetas: KerasWeights, evaluator: Evaluator
) -> Tuple[np.ndarray, KerasWeights, float, float]:
    weighting = random_weighting(len(thetas))
    theta_prime_candidate = weighted_federated_averaging(thetas, weighting)
    loss, acc = evaluator.evaluate(theta_prime_candidate)
    return weighting, theta_prime_candidate, loss, acc


def random_weighting(num_weightings: int, low=0.5, high=1.5) -> np.ndarray:
    return np.random.uniform(low=low, high=high, size=(num_weightings,))
