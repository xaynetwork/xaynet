from typing import Callable, Dict, List, Optional, Tuple

import numpy as np
from numpy import ndarray

from xain_fl.types import FederatedDataset, KerasDataset

from .transformer import classes_balanced_randomized_per_partition, sort_by_class


def load(keras_dataset) -> KerasDataset:
    """Loads Keras Dataset in predictable form

    Args:
        keras_dataset (Dataset)

    Returns:
        Dataset
    """
    (x_train, y_train), (x_test, y_test) = keras_dataset.load_data()

    y_train = y_train.reshape((y_train.shape[0],))
    y_test = y_test.reshape((y_test.shape[0],))

    return (x_train, y_train), (x_test, y_test)


def extract_validation_set(x: ndarray, y: ndarray, size=6000):
    """Will extract a validation set of "size" from given x,y pair.

    Args:
        x (ndarray): numpy array
        y (ndarray): numpy array
        size (int): Size of validation set. Must be smaller than examples count
                    in x, y and multiple of label_count
    """
    assert x.shape[0] == y.shape[0]
    assert (
        x.shape[0] % size == 0
    ), f"number of examples ({x.shape[0]}) needs to be evenly divisible by parameter size ({size})"

    assert size % len(set(y)) == 0, "size must be a multiple of number of labels"

    x_balanced, y_balanced = classes_balanced_randomized_per_partition(x, y)

    xy_val = (x_balanced[:size], y_balanced[:size])
    xy_train = (x_balanced[size:], y_balanced[size:])

    return xy_train, xy_val


def assert_is_balanced(y):
    [_, counts] = np.unique(y, return_counts=True)
    assert len(set(counts)) == 1, "Some classes appear more often than others"


def remove_balanced(x: ndarray, y: ndarray, num_remove: int) -> Tuple[ndarray, ndarray]:
    assert_is_balanced(y)

    num_classes = len(np.unique(y))
    num_remove_per_class = num_remove // num_classes

    assert (
        num_remove % num_classes == 0
    ), "Number of examples to be removed has to be divisible by num_remove"

    x, y = sort_by_class(x, y)

    x_splits = np.split(x, indices_or_sections=num_classes, axis=0)
    y_splits = np.split(y, indices_or_sections=num_classes, axis=0)

    x = np.concatenate([x_split[num_remove_per_class:] for x_split in x_splits])
    y = np.concatenate([y_split[num_remove_per_class:] for y_split in y_splits])

    return x, y


def create_federated_dataset(
    keras_dataset,
    num_partitions: int,
    validation_set_size: int,
    transformers: List[Callable],
    transformers_kwargs: Optional[Dict] = None,
) -> FederatedDataset:
    (x_train, y_train), xy_test = load(keras_dataset)

    if isinstance(num_partitions, list):
        assert x_train.shape[0] % (len(num_partitions) + 1) == 0
    else:
        assert x_train.shape[0] % num_partitions == 0

    (x_train, y_train), xy_val = extract_validation_set(
        x_train, y_train, size=validation_set_size
    )

    for i, transformer in enumerate(transformers):
        if (
            not transformers_kwargs
            or not transformers_kwargs[i]
            or transformers_kwargs[i] is None
        ):
            x_train, y_train = transformer(x_train, y_train)
        else:
            x_train, y_train = transformer(x_train, y_train, **transformers_kwargs[i])

    x_splits = np.split(x_train, indices_or_sections=num_partitions, axis=0)
    y_splits = np.split(y_train, indices_or_sections=num_partitions, axis=0)

    xy_splits = list(zip(x_splits, y_splits))

    return xy_splits, xy_val, xy_test
