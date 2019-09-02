from typing import Callable, Dict

import tensorflow as tf

from .blog_cnn import blog_cnn_compiled
from .orig_2nn import orig_2nn_compiled
from .orig_cnn import orig_cnn_compiled
from .resnet import resnet20v2_compiled

model_fns: Dict[str, Callable[[], tf.keras.Model]] = {
    "orig_2nn": orig_2nn_compiled,
    "orig_cnn": orig_cnn_compiled,
    "blog_cnn": blog_cnn_compiled,
    "resnet20": resnet20v2_compiled,
}


def load_model_fn(model_name: str) -> Callable[[], tf.keras.Model]:
    assert (
        model_name in model_fns
    ), f"Model name '{model_name}' not in {model_fns.keys()}"
    return model_fns[model_name]
