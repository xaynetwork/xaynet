from typing import Callable

import tensorflow as tf


class ModelProvider:
    def __init__(self, model_fn: Callable[[], tf.keras.Model], lr_fn_fn: Callable):
        self.init_model = model_fn
        self.init_lr_fn = lr_fn_fn
