import logging
import pickle
from typing import List, Optional

import numpy as np

from .aggregator import AggregatorABC

LOG = logging.getLogger("PythonWeightedAverageAggregator")


class Aggregator(AggregatorABC):
    def __init__(self):
        LOG.info("initializing aggregator")
        self.global_weights: np.ndarray = None
        self.weights: List[np.ndarray] = []
        self.aggregation_data: List[int] = []

    def add_weights(self, data: bytes) -> bool:
        LOG.info("adding weights (len = %d)", len(data))

        number_of_samples = int.from_bytes(data[:4], byteorder="big")
        weights = pickle.loads(data[4:])

        self.aggregation_data.append(number_of_samples)
        self.weights.append(weights)

        return True

    def aggregate(self) -> bytes:
        LOG.info("starting aggregation (%d models)", len(self.weights))
        aggregation_weights: np.ndarray
        if any(self.aggregation_data):
            aggregation_weights = np.array(self.aggregation_data) / np.sum(
                self.aggregation_data
            )
        else:
            aggregation_weights = np.ones_like(self.aggregation_data) / len(
                self.aggregation_data
            )

        scaled_model_weights: List[np.ndarray] = [
            model_weight * aggregation_weight
            for model_weight, aggregation_weight in zip(
                self.weights, aggregation_weights
            )
        ]
        self.global_weights = np.sum(scaled_model_weights, axis=0)
        self.weights = []
        self.aggregation_data = []
        LOG.info("finished aggregation")
        return self.get_global_weights()

    def reset(self, global_weights: Optional[bytes]) -> None:
        LOG.info("resetting aggregator")
        if global_weights is None:
            self.global_weights = None
        else:
            self.global_weights = pickle.loads(global_weights)
        self.weights = []
        self.aggregation_data = []

    def get_global_weights(self) -> bytes:
        LOG.info("returning global weights")
        data = pickle.dumps(self.global_weights)
        return data
