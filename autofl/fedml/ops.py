from typing import List

import tensorflow as tf
from numpy import ndarray


# FIXME align return type with KerasWeights type
def get_model_params(model: tf.keras.Model) -> List[List[ndarray]]:
    theta = []
    for layer in model.layers:
        theta.append(layer.get_weights())
    return theta


# FIXME align return type with KerasWeights type
def set_model_params(model: tf.keras.Model, theta: List[List[ndarray]]):
    for layer, layer_weights in zip(model.layers, theta):
        layer.set_weights(layer_weights)
