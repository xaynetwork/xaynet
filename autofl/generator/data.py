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
    permutation = np.random.RandomState(seed=SEED).permutation(x.shape[0])
    x_shuffled = x[permutation]
    y_shuffled = y[permutation]
    return x_shuffled, y_shuffled


def balanced_labels_shuffle(
    x: ndarray, y: ndarray, section_count=10
) -> Tuple[ndarray, ndarray]:
    """Shuffled y so that the labels are uniformly distributed in each section"""
    assert x.shape[0] == y.shape[0], "x and y need to have them dimension on axis=0"
    assert (
        np.unique(y).shape[0] % section_count == 0
    ), "count of unique labels needs to be divideable by section_count"

    samples_count = y.shape[0]

    assert (
        samples_count % section_count == 0
    ), "Number of examples needs to be evenly divideable by section_count"

    # Number of samples per section e.g. for 60 samples and section_count=10 => 6
    section_size = int(samples_count / section_count)

    # Array of indices that sort a along the specified axis.
    index_array = np.argsort(y, axis=0)

    x_sorted = x[index_array]
    y_sorted = y[index_array]

    # Create a permutation which will be the basis to shuffle each section in itself
    global_permutation = np.array([], dtype=np.int64)

    # To create a global permutation we will first shift each entry
    # in our permutation for each section by section_size
    section_permutations = [
        list(
            map(
                lambda x: x + (i * section_size),
                # Use as seed = SEED + i so each section is uniquly shuffled
                np.random.RandomState(seed=SEED + i).permutation(section_size),
            )
        )
        for i in range(section_count)
    ]

    global_permutation = np.append(global_permutation, section_permutations)

    x_shuffled = x[global_permutation]
    y_shuffled = y[global_permutation]

    return x_shuffled, y_shuffled

    # 1. sort by labels
    # 2. shuffle labels groups in themself
    # 3. take into each split 600 from each label


def split(
    x: ndarray, y: ndarray, num_splits: int
) -> Tuple[List[ndarray], List[ndarray]]:
    x_splits = np.split(x, indices_or_sections=num_splits, axis=0)
    y_splits = np.split(y, indices_or_sections=num_splits, axis=0)
    return x_splits, y_splits


def generate_splits(
    num_splits: int, keras_dataset, shuffle_method=random_shuffle
) -> FederatedDataset:
    (x_train, y_train), (x_test, y_test) = load(keras_dataset)

    assert x_train.shape[0] % num_splits == 0

    x_train, y_train = shuffle_method(x_train, y_train)

    x_splits, y_splits = split(x_train, y_train, num_splits)

    xy_splits = list(zip(x_splits, y_splits))

    return xy_splits, (x_test, y_test)


def generate_splits_mnist(num_splits: int) -> FederatedDataset:
    return generate_splits(num_splits, tf.keras.datasets.mnist)
