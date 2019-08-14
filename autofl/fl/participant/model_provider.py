from typing import Callable

import tensorflow as tf


class ModelProvider:
    def __init__(self, model_fn: Callable[[], tf.keras.Model]):
        self.init_model = model_fn
