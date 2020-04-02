from io import BytesIO
import logging
from typing import List, Optional

import numpy as np

from .aggregator import AggregatorABC

LOG = logging.getLogger("PythonWeightedAverageAggregator")


class Aggregator(AggregatorABC):
    def __init__(self) -> None:
        logging.basicConfig(level=logging.INFO)
        LOG.info("initializing aggregator")
        self.global_weights: np.ndarray = None
        self.weights: List[np.ndarray] = []
        self.aggregation_data: List[int] = []

    def add_weights(self, data: bytes) -> bool:
        LOG.info("adding weights (len = %d)", len(data))

        number_of_samples = int.from_bytes(data[:4], byteorder="big")
        weights = np.load(BytesIO(data[4:]), allow_pickle=False)

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
        # If global_weights is a scalar, make it a one dimensional
        # array
        if self.global_weights.shape == ():
            self.global_weights = np.array([self.global_weights])
        self.weights = []
        self.aggregation_data = []
        LOG.info("finished aggregation")
        return self.get_global_weights()

    def reset(self, global_weights: Optional[bytes]) -> None:
        LOG.info("resetting aggregator")
        if global_weights is None:
            self.global_weights = None
        else:
            reader = BytesIO(global_weights)
            self.global_weights = np.load(reader, allow_pickle=False)

        self.weights = []
        self.aggregation_data = []

    def get_global_weights(self) -> bytes:
        LOG.info("returning global weights")
        if self.global_weights is None:
            return b""
        writer = BytesIO()
        np.save(writer, self.global_weights, allow_pickle=False)
        # We cannot use getvalue here because it copies the buffer.
        # getbuffer will not as long as the data is not modified.
        return writer.getbuffer()[:]
