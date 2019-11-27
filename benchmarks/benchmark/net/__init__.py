"""Provides implementations of a few standard neural network architectures and associated
learning rate schedules.
"""
from typing import Callable, Dict

import tensorflow as tf

from .blog_cnn import blog_cnn_compiled, blog_cnn_lr_fn
from .orig_2nn import orig_2nn_compiled, orig_2nn_lr_fn
from .orig_cnn import orig_cnn_compiled, orig_cnn_lr_fn
from .resnet import resnet20v2_compiled, resnet20v2_lr_fn

model_fns: Dict[str, Callable[[], tf.keras.Model]] = {
    "orig_2nn": orig_2nn_compiled,
    "orig_cnn": orig_cnn_compiled,
    "blog_cnn": blog_cnn_compiled,
    "resnet20": resnet20v2_compiled,
}

lr_fns: Dict = {
    "orig_2nn": orig_2nn_lr_fn,
    "orig_cnn": orig_cnn_lr_fn,
    "blog_cnn": blog_cnn_lr_fn,
    "resnet20": resnet20v2_lr_fn,
}


def load_model_fn(model_name: str) -> Callable[[], tf.keras.Model]:
    """Returns a function which can be used to create a Keras model.

    Args:
        model_name (str): One of ~benchmarks.benchmark.net.model_fns

    Returns:
        Callable[[], tf.keras.Model]
    """
    assert (
        model_name in model_fns
    ), f"Model name '{model_name}' not in {model_fns.keys()}"
    return model_fns[model_name]


def load_lr_fn_fn(model_name: str) -> Callable:
    """Returns a function which can be used to obtain a learning rate schedule.

    Args:
        model_name (str): One of ~benchmarks.benchmark.net.lr_fns

    Returns:
        Callable
    """
    assert model_name in lr_fns, f"Model name '{model_name}' not in {lr_fns.keys()}"
    return lr_fns[model_name]
