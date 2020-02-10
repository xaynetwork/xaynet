"""Provides an abstract base class Aggregator and multiple sub-classes"""

from abc import ABC, abstractmethod
from typing import List

import numpy as np
from numpy import ndarray
from structlog import get_logger

from xain_fl.logger import StructLogger

logger: StructLogger = get_logger(__name__)


class Aggregator(ABC):  # pylint: disable=too-few-public-methods
    """Abstract base class for model weights aggregation strategies."""

    def __init__(self):
        """Initialize the aggregator."""

    @abstractmethod
    def aggregate(
        self, multiple_model_weights: List[ndarray], aggregation_data: List[int]
    ) -> ndarray:
        """Aggregate the weights of multiple models.

        Args:
            multiple_model_weights (List[ndarray]): The weights of multiple models.
            aggregation_data (List[int]): Meta data for model aggregation.

        Returns:
            ndarray: The aggregated model weights.
        """

        raise NotImplementedError()


class IdentityAggregator(Aggregator):  # pylint: disable=too-few-public-methods
    """Identity aggregation."""

    def aggregate(
        self, multiple_model_weights: List[ndarray], aggregation_data: List[int]
    ) -> ndarray:
        """Identity aggregation only for one set of model weights.

        Args:
            multiple_model_weights (List[ndarray]): The weights of multiple models. Must have only
                one set of weights.
            aggregation_data (List[int]): Meta data for model aggregation. Not used here.

        Returns:
            ndarray: The identical model weights.

        Raises:
            ValueError: If more than one set of model weights is provided.
        """

        if len(multiple_model_weights) > 1:
            raise ValueError("Invalid number of model weights!")

        return multiple_model_weights[0]


class ModelSumAggregator(Aggregator):  # pylint: disable=too-few-public-methods
    """Summation of models aggregation."""

    def aggregate(
        self, multiple_model_weights: List[ndarray], aggregation_data: List[int]
    ) -> ndarray:
        """Aggregate the weights of multiple models by summation.

        Args:
            multiple_model_weights (List[ndarray]): The weights of multiple models.
            aggregation_data (List[int]): Meta data for model aggregation. Not used here.

        Returns:
            ndarray: The aggregated model weights.
        """

        return sum(multiple_model_weights)


class WeightedAverageAggregator(Aggregator):  # pylint: disable=too-few-public-methods
    """Weighted average aggregation."""

    def aggregate(
        self, multiple_model_weights: List[ndarray], aggregation_data: List[int]
    ) -> ndarray:
        """Aggregate the weights of multiple models by weighted averages.

        Proposed by McMahan et al in: https://arxiv.org/abs/1602.05629

        Args:
            multiple_model_weights (List[ndarray]): The weights of multiple models.
            aggregation_data (List[int]): Meta data for model aggregation. Here it is expected to be
                the number of train samples per set of model weights. If all numbers are zero, e.g.
                in a 0th round for weight initialization, then all model weights are weighted
                equally.

        Returns:
            ndarray: The aggregated model weights.
        """

        # get aggregation weights
        aggregation_weights: ndarray
        if any(aggregation_data):
            aggregation_weights = np.array(aggregation_data) / np.sum(aggregation_data)
        else:
            aggregation_weights = np.ones_like(aggregation_data) / len(aggregation_data)

        # compute weighted average
        scaled_model_weights: List[ndarray] = [
            model_weight * aggregation_weight
            for model_weight, aggregation_weight in zip(
                multiple_model_weights, aggregation_weights
            )
        ]
        aggregated_model_weights: ndarray = np.sum(scaled_model_weights, axis=0)

        return aggregated_model_weights


# TODO: (XP-351) decide how to continue with that
# def federated_averaging(multiple_model_weights: List[ndarray], weighting: ndarray) -> ndarray:
#     """Calculates weighted averages of provided list of weight updates, as proposed by McMahan et
#         al. in: https://arxiv.org/abs/1602.05629
#
#     Args:
#         multiple_model_weights (List[ndarray]): List of model weight.
#         weighting (ndarray): Describes relative weight of each model weight. Required to be the
#             same length as argument multiple_model_weights.
#
#     Returns:
#         ndarray: The aggregated model weights.
#     """
#
#     assert weighting.ndim == 1
#     assert len(multiple_model_weights) == weighting.shape[0]
#
#     model_weights_avg: ndarray = multiple_model_weights[0] * weighting[0]
#
#     # Aggregate (weighted) updates
#     for weights, update_weighting in zip(multiple_model_weights[1:], weighting[1:]):
#         model_weights_avg += update_weighting * weights
#
#     weighting_sum = np.sum(weighting)
#     model_weights_avg /= weighting_sum
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

# def evo_agg(weight_updates: List[ndarray], evaluator: Evaluator) -> ndarray:
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
# ) -> Tuple[ndarray, ndarray, float, float]:
#     weighting = _random_weighting(len(weight_updates))
#     weights_prime_candidate = federated_averaging(weight_updates, weighting)
#     loss, acc = evaluator.evaluate(weights_prime_candidate)
#     return weighting, weights_prime_candidate, loss, acc


# def _random_weighting(num_weightings: int, low=0.5, high=1.5) -> ndarray:
#     return np.random.uniform(low=low, high=high, size=(num_weightings,))
