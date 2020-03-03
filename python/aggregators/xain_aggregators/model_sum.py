import logging
import pickle
from typing import List, Optional

import numpy as np

from .aggregator import AggregatorABC

LOG = logging.getLogger("PythonModelSumAggregator")


class Aggregator(AggregatorABC):
    def __init__(self):
        LOG.info("initializing aggregator")
        self.global_weights: np.ndarray = None
        self.weights: List[np.ndarray] = []

    def add_weights(self, data: bytes) -> bool:
        LOG.info("adding weights (len = %d)", len(data))
        # FIXME: perform some checks here
        weights = pickle.loads(data)
        self.weights.append(weights)
        return True

    def aggregate(self) -> bytes:
        LOG.info("starting aggregation (%d models)", len(self.weights))
        self.global_weights = sum(self.weights)
        LOG.info("finished aggregation")
        return self.get_global_weights()

    def reset(self, global_weights: Optional[bytes]) -> None:
        LOG.info("resetting aggregator")
        if global_weights is None:
            self.global_weights = None
        else:
            self.global_weights = pickle.loads(global_weights)
        self.weights = []

    def get_global_weights(self) -> bytes:
        LOG.info("returning global weights")
        data = pickle.dumps(self.global_weights)
        return data
