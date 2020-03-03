from .aggregator import AggregatorABC
from typing import Optional
import numpy as np
import pickle

DUMMY_WEIGHTS = np.ndarray([1, 2, 3])


class Aggregator(AggregatorABC):

    def __init__(self):
        self.global_weights = DUMMY_WEIGHTS
        self.weights = []

    def add_weights(self, data: bytes) -> bool:
        weights = pickle.loads(data)
        self.weights.append(weights)
        return True

    def aggregate(self) -> bytes:
        # Do nothing for now, just return the global weights
        data = pickle.dumps(self.global_weights)
        return data

    def reset(self, global_weights: Optional[np.ndarray]) -> None:
        if global_weights is None:
            global_weights = DUMMY_WEIGHTS
        self.weights = []

    def get_global_weights(self) -> np.ndarray:
        data = pickle.dumps(self.global_weights)
        return data
