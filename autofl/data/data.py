import os
from typing import List, Optional, Tuple

import numpy as np
import tensorflow as tf
from numpy import ndarray

os.environ["TF_CPP_MIN_LOG_LEVEL"] = "3"
tf.logging.set_verbosity(tf.logging.ERROR)


def main():
    # Load dataset
    x_train, y_train, _, _ = load(tf.keras.datasets.cifar10)
    print("Training set before split:")
    print("\tx_train:", x_train.shape, type(x_train))
    print("\ty_train:", y_train.shape, type(x_train))

    # Shuffle both x and y with the same permutation
    x_train, y_train = shuffle(x_train, y_train)
    print("Training set after shuffle:")
    print("\tx_train:", x_train.shape, type(x_train))
    print("\ty_train:", y_train.shape, type(x_train))

    x_splits, y_splits = split(x_train, y_train, num_splits=5)
    print("Training set after split:")
    for i, (x_split, y_split) in enumerate(zip(x_splits, y_splits)):
        print("\t", str(i), "x_split:", x_split.shape, type(x_split))
        print("\t", str(i), "y_split:", y_split.shape, type(y_split))


def load(keras_dataset) -> Tuple[ndarray, ndarray, ndarray, ndarray]:
    (x_train, y_train), (x_test, y_test) = keras_dataset.load_data()
    return x_train, y_train, x_test, y_test


def shuffle(
    x: ndarray, y: ndarray, seed: Optional[int] = None
) -> Tuple[ndarray, ndarray]:
    assert x.shape[0] == y.shape[0]
    permutation = np.random.RandomState(seed=seed).permutation(x.shape[0])
    x_shuffled = x[permutation]
    y_shuffled = y[permutation]
    return x_shuffled, y_shuffled


def split(
    x: ndarray, y: ndarray, num_splits: int
) -> Tuple[List[ndarray], List[ndarray]]:
    x_splits = np.split(x, indices_or_sections=num_splits, axis=0)
    y_splits = np.split(y, indices_or_sections=num_splits, axis=0)
    return x_splits, y_splits


def load_splits(
    num_splits: int, keras_dataset
) -> Tuple[List[ndarray], List[ndarray], ndarray, ndarray]:
    x_train, y_train, x_test, y_test = load(keras_dataset)
    assert x_train.shape[0] % num_splits == 0
    x_train, y_train = shuffle(x_train, y_train)
    x_splits, y_splits = split(x_train, y_train, num_splits)
    return x_splits, y_splits, x_test, y_test


def load_splits_cifar10(
    num_splits: int
) -> Tuple[List[ndarray], List[ndarray], ndarray, ndarray]:
    return load_splits(num_splits, tf.keras.datasets.cifar10)


def load_splits_mnist(
    num_splits: int
) -> Tuple[List[ndarray], List[ndarray], ndarray, ndarray]:
    return load_splits(num_splits, tf.keras.datasets.mnist)
