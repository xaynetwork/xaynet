from typing import List

import tensorflow as tf
from numpy import ndarray


def federated_averaging(thetas: List[List[List[ndarray]]]) -> List[List[ndarray]]:
    theta_avg: List[List[ndarray]] = thetas[0]
    for theta in thetas[1:]:
        for layer_index, layer in enumerate(theta):
            for weight_index, weights in enumerate(layer):
                theta_avg[layer_index][weight_index] += weights
    for layer in theta_avg:
        for weights in layer:
            weights /= len(thetas)
    return theta_avg


def get_model_params(model: tf.keras.Model) -> List[List[ndarray]]:
    theta = []
    for layer in model.layers:
        theta.append(layer.get_weights())
    return theta


def set_model_params(model: tf.keras.Model, theta: List[List[ndarray]]):
    for layer, layer_weights in zip(model.layers, theta):
        layer.set_weights(layer_weights)
