"""Provides an abstract base class Aggregator and multiple sub-classes"""

from abc import ABC, abstractmethod
from typing import List, Tuple

import numpy as np
from numpy import ndarray

from xain_fl.logger import StructLogger, get_logger

logger: StructLogger = get_logger(__name__)


class Aggregator(ABC):  # pylint: disable=too-few-public-methods
    """Abstract base class for model weights aggregation strategies."""

    def __init__(self):
        """Initialize the aggregator."""

    @abstractmethod
    def aggregate(self, weight_updates: List[Tuple[List[ndarray], int]]) -> List[ndarray]:
        """Aggregate the weights of multiple models.

        Args:
            weight_updates (List[Tuple[List[ndarray], int]]): Pairs of model weights and number of
                training samples.

        Returns:
            List[ndarray]: The aggregated model weights.
        """

        raise NotImplementedError()


class IdentityAgg(Aggregator):  # pylint: disable=too-few-public-methods
    """Identity aggregation."""

    def aggregate(self, weight_updates: List[Tuple[List[ndarray], int]]) -> List[ndarray]:
        """Identity aggregation only for weight updates of length one.

        Args:
            weight_updates (List[Tuple[List[ndarray], int]]): A single pairs of model weights and
                number of training samples.

        Returns:
            List[ndarray]: The identical model weights.
        """

        assert len(weight_updates) == 1
        return weight_updates[0][0]


class ModelSumAgg(Aggregator):  # pylint: disable=too-few-public-methods
    """Sum-of-models aggregation."""

    def aggregate(self, weight_updates: List[Tuple[List[ndarray], int]]) -> List[ndarray]:
        """Aggregate the weights of multiple models by summation.

        Args:
            weight_updates (List[Tuple[List[ndarray], int]]): Pairs of model weights and number of
                training samples.

        Returns:
            List[ndarray]: The aggregated model weights.
        """

        return [sum(weights) for weights, _ in weight_updates]


class FederatedAveragingAgg(Aggregator):  # pylint: disable=too-few-public-methods
    """Weighted average aggregation."""

    def aggregate(self, weight_updates: List[Tuple[List[ndarray], int]]) -> List[ndarray]:
        """Aggregate the weights of multiple models by weighted averages.

        Proposed by McMahan et al in: https://arxiv.org/abs/1602.05629

        Args:
            weight_updates (List[Tuple[List[ndarray], int]]): Pairs of model weights and number of
                training samples.

        Returns:
            List[ndarray]: The aggregated model weights.

        Raises:
            ValueError: If the total number of training samples is zero.
        """

        aggregation_weights: List[int] = [number_samples for _, number_samples in weight_updates]
        aggregation_weights_sum: ndarray = np.sum(aggregation_weights)
        if not aggregation_weights_sum:
            raise ValueError("Invalid total number of training samples!")

        model_weights: List[List[ndarray]] = [weights for weights, _ in weight_updates]

        aggregated_model_weights: List[ndarray] = [
            np.sum([model_weight * aggregation_weight for model_weight in model_weights_per_layer])
            for model_weights_per_layer, aggregation_weight in zip(
                zip(*model_weights), aggregation_weights
            )
        ]

        return aggregated_model_weights


# TODO: (XP-351) decide how to continue with that
# def federated_averaging(
#     model_weights: List[List[ndarray]], weighting: ndarray
# ) -> List[ndarray]:
#     """Calculates weighted averages of provided list of weight updates, as proposed by McMahan et
#         al. in: https://arxiv.org/abs/1602.05629
#
#     Args:
#         model_weights (List[List[ndarray]]): List of model weight.
#         weighting (ndarray): Describes relative weight of each model weight. Required to be the
#             same length as argument model_weights.
#
#     Returns:
#         List[ndarray]: The aggregated model weights.
#     """
#
#     assert weighting.ndim == 1
#     assert len(model_weights) == weighting.shape[0]
#
#     model_weights_avg: List[ndarray] = model_weights[0]
#     for weights in model_weights_avg:
#         weights *= weighting[0]
#
#     # Aggregate (weighted) updates
#     for weights, update_weighting in zip(model_weights[1:], weighting[1:]):
#         for w_index, weight in enumerate(weights):
#             model_weights_avg[w_index] += update_weighting * weight
#
#     weighting_sum = np.sum(weighting)
#     for weights in model_weights_avg:
#         weights /= weighting_sum
#
#     return model_weights_avg

# class EvoAgg(Aggregator):
#     """Experimental"""
#
#     def __init__(self, evaluator: Evaluator):
#         super().__init__()
#         self.evaluator = evaluator
#
#     def aggregate(self, weight_updates: List[Tuple[List[ndarray], int]]) -> List[ndarray]:
#         weight_matrices = [model_weights for model_weights, num_examples in weight_updates]
#         return evo_agg(weight_matrices, self.evaluator)

# def evo_agg(weight_updates: List[List[ndarray]], evaluator: Evaluator) -> List[ndarray]:
#     """Experimental
#
#         - Init different weightings
#         - Aggregate weight updates according to those weightings ("candidates")
#         - Evaluate all candidates on the validation set
#         - Pick (a) best candidate, or (b) average of n best candidates
#     """
#     # Compute candidates
#     # TODO in parallel, do:
#     weights_prime_candidates = []
#     for _ in range(3):
#         candidate = _compute_candidate(weight_updates, evaluator)
#         logger.debug("Candidate metadata", weighting=candidate[0], loss=candidate[2])
#
#         weights_prime_candidates.append(candidate)
#     # Return best candidate
#     best_candidate = _pick_best_candidate(weights_prime_candidates)
#     return best_candidate


# def _pick_best_candidate(candidates: List) -> List[ndarray]:
#     _, best_candidate, best_loss, _ = candidates[0]
#     for _, candidate, loss, _ in candidates[1:]:
#         if loss < best_loss:
#             best_candidate = candidate
#             best_loss = loss
#     return best_candidate


# def _compute_candidate(
#     weight_updates: List[ndarray], evaluator: Evaluator
# ) -> Tuple[ndarray, List[ndarray], float, float]:
#     weighting = _random_weighting(len(weight_updates))
#     weights_prime_candidate = federated_averaging(weight_updates, weighting)
#     loss, acc = evaluator.evaluate(weights_prime_candidate)
#     return weighting, weights_prime_candidate, loss, acc


# def _random_weighting(num_weightings: int, low=0.5, high=1.5) -> ndarray:
#     return np.random.uniform(low=low, high=high, size=(num_weightings,))
