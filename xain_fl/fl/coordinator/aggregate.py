"""Provides an abstract base class Aggregator and multiple sub-classes
such as FederatedAveragingAgg.
"""
from abc import ABC, abstractmethod
from typing import List, Tuple

import numpy as np

from xain_fl.fl.types import Theta
from xain_fl.logger import StructLogger, get_logger

logger: StructLogger = get_logger(__name__)


class Aggregator(ABC):  # pylint: disable=too-few-public-methods
    """Abstract base class which provides an interface to the coordinator that
    enables different aggregation implementations.
    """

    def __init__(self):
        """[summary]

        .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
        """

    @abstractmethod
    def aggregate(self, thetas: List[Tuple[Theta, int]]) -> Theta:
        """Aggregates given a list of thetas and returns the aggregate.

        Args:
            thetas (List[Tuple[Theta, int]]): List of tuples with theta and the number
            of examples used to obtain theta.

        Returns:
            Theta
        """
        raise NotImplementedError()


class IdentityAgg(Aggregator):  # pylint: disable=too-few-public-methods
    """Provides identity aggregation, i.e. the aggregate method expects
    a list containing a single element and returns that element.
    """

    def aggregate(self, thetas: List[Tuple[Theta, int]]) -> Theta:
        """Accepts only a thetas list of length one."""
        assert len(thetas) == 1
        return thetas[0][0]


class ModelSumAgg(Aggregator):  # pylint: disable=too-few-public-methods
    """Provides a sum-of-models aggregation."""

    def aggregate(self, thetas: List[Tuple[Theta, int]]) -> Theta:
        """Aggregates a given list of models by summation.

        Args:
            thetas (~typing.List[~xain_fl.fl.types.Theta]): List of thetas.

        Returns:
            ~xain_fl.fl.types.Theta: The aggregated model weights.
        """
        return [sum(th) for th, _ in thetas]


class FederatedAveragingAgg(Aggregator):  # pylint: disable=too-few-public-methods
    """Provides federated averaging aggregation, i.e. a weighted average."""

    def aggregate(self, thetas: List[Tuple[Theta, int]]) -> Theta:
        theta_list = [theta for theta, _ in thetas]
        weighting = np.array([num_examples for _, num_examples in thetas])
        return federated_averaging(theta_list, weighting)


def federated_averaging(thetas: List[Theta], weighting: np.ndarray) -> Theta:
    """Calculates weighted averages of provided list of thetas, as proposed by McMahan et al. in:
        https://arxiv.org/abs/1602.05629

    Args:
        thetas (List[Theta]): List of thetas.
        weighting (np.ndarray): Describes relative weight of each theta. Required to be the
            same length as argument thetas.

    Returns:
        Theta
    """
    assert weighting.ndim == 1
    assert len(thetas) == weighting.shape[0]

    theta_avg: Theta = thetas[0]
    for weights in theta_avg:
        weights *= weighting[0]

    # Aggregate (weighted) updates
    for theta, update_weighting in zip(thetas[1:], weighting[1:]):
        for w_index, weights in enumerate(theta):
            theta_avg[w_index] += update_weighting * weights

    weighting_sum = np.sum(weighting)
    for weights in theta_avg:
        weights /= weighting_sum

    return theta_avg


# TODO: (XP-351) decide how to continue with that
# class EvoAgg(Aggregator):
#     """Experimental"""

#     def __init__(self, evaluator: Evaluator):
#         super().__init__()
#         self.evaluator = evaluator

#     def aggregate(self, thetas: List[Tuple[Theta, int]]) -> Theta:
#         weight_matrices = [theta for theta, num_examples in thetas]
#         return evo_agg(weight_matrices, self.evaluator)

# def evo_agg(thetas: List[Theta], evaluator: Evaluator) -> Theta:
#     """Experimental

#         - Init different weightings
#         - Aggregate thetas according to those weightings ("candidates")
#         - Evaluate all candidates on the validation set
#         - Pick (a) best candidate, or (b) average of n best candidates
#     """
#     # Compute candidates
#     # TODO in parallel, do:
#     theta_prime_candidates = []
#     for _ in range(3):
#         candidate = _compute_candidate(thetas, evaluator)
#         logger.debug("Candidate metadata", weighting=candidate[0], loss=candidate[2])

#         theta_prime_candidates.append(candidate)
#     # Return best candidate
#     best_candidate = _pick_best_candidate(theta_prime_candidates)
#     return best_candidate


# def _pick_best_candidate(candidates: List) -> Theta:
#     _, best_candidate, best_loss, _ = candidates[0]
#     for _, candidate, loss, _ in candidates[1:]:
#         if loss < best_loss:
#             best_candidate = candidate
#             best_loss = loss
#     return best_candidate


# def _compute_candidate(
#     thetas: Theta, evaluator: Evaluator
# ) -> Tuple[np.ndarray, Theta, float, float]:
#     weighting = _random_weighting(len(thetas))
#     theta_prime_candidate = federated_averaging(thetas, weighting)
#     loss, acc = evaluator.evaluate(theta_prime_candidate)
#     return weighting, theta_prime_candidate, loss, acc


# def _random_weighting(num_weightings: int, low=0.5, high=1.5) -> np.ndarray:
#     return np.random.uniform(low=low, high=high, size=(num_weightings,))
