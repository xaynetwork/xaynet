from typing import List, Tuple

import numpy as np
import tensorflow as tf
from numpy import ndarray

from autofl.types import FederatedDataset, KerasDataset

# Passed to RandomState for predictable shuffling
SEED = 851746


def load(keras_dataset) -> KerasDataset:
    (x_train, y_train), (x_test, y_test) = keras_dataset.load_data()

    y_train = y_train.reshape((y_train.shape[0],))
    y_test = y_test.reshape((y_test.shape[0],))

    return (x_train, y_train), (x_test, y_test)


def random_shuffle(x: ndarray, y: ndarray) -> Tuple[ndarray, ndarray]:
    assert x.shape[0] == y.shape[0]
    # pylint: disable-msg=no-member
    permutation = np.random.RandomState(seed=SEED).permutation(x.shape[0])
    x_shuffled = x[permutation]
    y_shuffled = y[permutation]
    return x_shuffled, y_shuffled


def balanced_labels_shuffle(
    x: ndarray, y: ndarray, section_count=10
) -> Tuple[ndarray, ndarray]:
    """Shuffled y so that the labels are uniformly distributed in each section"""
    assert x.shape[0] == y.shape[0], "x and y need to have them dimension on axis=0"

    example_count = y.shape[0]
    unique_label_count = np.unique(y).shape[0]
    section_size = int(example_count / section_count)

    assert (
        unique_label_count % section_count == 0
    ), "count of unique labels needs to be divideable by section_count"

    assert (
        example_count % section_count == 0
    ), "Number of examples needs to be evenly divideable by section_count"

    x_shuffled, y_shuffled = random_shuffle(x, y)

    # Array of indices that sort a along the specified axis.
    sort_indexes = np.argsort(y_shuffled, axis=0)

    x_sorted = x_shuffled[sort_indexes]
    y_sorted = y_shuffled[sort_indexes]

    section_indicies = (
        np.array(range(example_count), np.int64)
        .reshape((section_size, section_count))
        .transpose()
        .reshape(example_count)
    )

    x_biased = x_sorted[section_indicies]
    y_biased = y_sorted[section_indicies]

    return x_biased, y_biased


def group_by_label(x: ndarray, y: ndarray) -> Tuple[ndarray, ndarray]:
    """
    Shuffles y so that only a single label is in each section
    Number of sections will depend on number of unique labels
    """
    assert x.shape[0] == y.shape[0], "x and y need to have them dimension on axis=0"

    example_count = y.shape[0]
    section_count = np.unique(y).shape[0]

    assert (
        example_count % section_count == 0
    ), "Number of examples needs to be evenly divideable by section_count"

    # Array of indices that sort a along the specified axis.
    sort_indexes = np.argsort(y, axis=0)

    x_sorted = x[sort_indexes]
    y_sorted = y[sort_indexes]

    return x_sorted, y_sorted


def split(
    x: ndarray, y: ndarray, num_splits: int
) -> Tuple[List[ndarray], List[ndarray]]:
    x_splits = np.split(x, indices_or_sections=num_splits, axis=0)
    y_splits = np.split(y, indices_or_sections=num_splits, axis=0)
    return x_splits, y_splits


def generate_splits(
    num_splits: int, keras_dataset, transformer=random_shuffle
) -> FederatedDataset:
    (x_train, y_train), (x_test, y_test) = load(keras_dataset)

    assert x_train.shape[0] % num_splits == 0

    x_train, y_train = transformer(x_train, y_train)

    x_splits, y_splits = split(x_train, y_train, num_splits)

    xy_splits = list(zip(x_splits, y_splits))

    return xy_splits, (x_test, y_test)


def generate_splits_mnist(num_splits: int) -> FederatedDataset:
    return generate_splits(num_splits, tf.keras.datasets.mnist)
