from io import BytesIO
import logging
from typing import List, Optional

import numpy as np

from .aggregator import AggregatorABC

LOG = logging.getLogger("PythonModelSumAggregator")


class Aggregator(AggregatorABC):
    def __init__(self) -> None:
        LOG.info("initializing aggregator")
        self.global_weights: np.ndarray = None
        self.weights: List[np.ndarray] = []

    def add_weights(self, data: bytes) -> bool:
        LOG.info("adding weights (len = %d)", len(data))
        reader = BytesIO(data)
        weights = np.load(reader, allow_pickle=False)
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
            reader = BytesIO(global_weights)
            self.global_weights = np.load(reader)
        self.weights = []

    def get_global_weights(self) -> bytes:
        LOG.info("returning global weights")
        writer = BytesIO()
        np.save(writer, self.global_weights, allow_pickle=False)
        # We cannot use getvalue here because it copies the buffer.
        # getbuffer will not as long as the data is not modified.
        return writer.getbuffer()[:]
