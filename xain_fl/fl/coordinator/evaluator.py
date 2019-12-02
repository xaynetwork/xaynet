from typing import Tuple

import tensorflow as tf

from xain_fl.datasets import prep
from xain_fl.types import Partition, Theta


class Evaluator:
    """Evaluates the model performance on a given data partition."""

    def __init__(self, model: tf.keras.Model, xy_val: Partition) -> None:
        self.model = model
        self.ds_val = prep.init_ds_val(xy_val)

    def evaluate(self, theta: Theta) -> Tuple[float, float]:
        self.model.set_weights(theta)
        # Assume the validation `tf.data.Dataset` to yield exactly one batch containing
        # all examples in the validation set
        loss, acc = self.model.evaluate(self.ds_val, steps=1)
        return loss, acc
