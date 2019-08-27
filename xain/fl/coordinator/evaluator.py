from typing import Tuple

import numpy as np
import tensorflow as tf

from xain.datasets import prep
from xain.types import KerasWeights


class Evaluator:
    def __init__(
        self, model: tf.keras.Model, xy_val: Tuple[np.ndarray, np.ndarray]
    ) -> None:
        self.model = model
        self.ds_val = prep.init_ds_val(xy_val)

    def evaluate(self, theta: KerasWeights) -> Tuple[float, float]:
        self.model.set_weights(theta)
        # Assume the validation `tf.data.Dataset` to yield exactly one batch containing
        # all examples in the validation set
        loss, acc = self.model.evaluate(self.ds_val, steps=1)
        return loss, acc
