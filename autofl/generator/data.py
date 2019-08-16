from typing import Callable, List, Tuple

import numpy as np
from numpy import ndarray

from autofl.types import FederatedDataset, KerasDataset

# Passed to RandomState for predictable shuffling
SEED = 851746


def load(keras_dataset) -> KerasDataset:
    (x_train, y_train), (x_test, y_test) = keras_dataset.load_data()

    y_train = y_train.reshape((y_train.shape[0],))
    y_test = y_test.reshape((y_test.shape[0],))

    return (x_train, y_train), (x_test, y_test)


def split(
    x: ndarray, y: ndarray, num_splits: int
) -> Tuple[List[ndarray], List[ndarray]]:
    x_splits = np.split(x, indices_or_sections=num_splits, axis=0)
    y_splits = np.split(y, indices_or_sections=num_splits, axis=0)
    return x_splits, y_splits


def extract_validation_set(x: ndarray, y: ndarray, size=6000):
    """Will extract a validation set of "size" from given x,y pair

    Parameters:
    x (ndarray): numpy array
    y (ndarray): numpy array
    size (int): Size of validation set. Must be smaller than examples count
                in x, y and multiple of label_count
    """
    assert x.shape[0] == y.shape[0]
    assert (
        x.shape[0] % size == 0
    ), "x.shape[0] (number of examples) needs to be evenly dividable by size"

    assert size % len(set(y)) == 0, "size must be a multiple of number of labels"

    x_balanced, y_balanced = balanced_labels_shuffle(x, y)

    xy_val = (x_balanced[:size], y_balanced[:size])
    xy_train = (x_balanced[size:], y_balanced[size:])

    return xy_train, xy_val


def assert_is_balanced(y):
    [_, counts] = np.unique(y, return_counts=True)
    assert len(set(counts)) == 1, "Some labels appear more often than others"


def take_balanced(x: ndarray, y: ndarray, num_take: int):
    assert_is_balanced(y)

    num_classes = len(np.unique(y))
    num_take_per_class = num_take // num_classes

    assert (
        num_take % num_classes == 0
    ), "Number of examples to be removed has to be divideable by num_take"

    x, y = group_by_label(x, y)

    x_splits, y_splits = split(x, y, num_classes)

    x = np.concatenate([x_split[num_take_per_class:] for x_split in x_splits])
    y = np.concatenate([y_split[num_take_per_class:] for y_split in y_splits])

    return x, y


def generate_splits(
    num_splits: int,
    keras_dataset,
    transformers,
    transformers_kwargs=None,
    validation_set_size=6000,
) -> FederatedDataset:
    (x_train, y_train), xy_test = load(keras_dataset)

    assert x_train.shape[0] % num_splits == 0

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

    x_splits, y_splits = split(x_train, y_train, num_splits)

    xy_splits = list(zip(x_splits, y_splits))

    return xy_splits, xy_val, xy_test


#####################################################
### From here on our transformers will be defined ###
#####################################################


def transfomer_decorator(func: Callable):
    """The decorator will validate the input and result of any
    transformer function it is applied to"""

    def wrapper(
        x: np.ndarray, y: np.ndarray, *args, **kwargs
    ) -> Tuple[np.ndarray, np.ndarray]:
        assert x.shape[0] == y.shape[0], "x and y need to have them dimension on axis=0"

        x_transformed, y_transformed = func(x, y, *args, **kwargs)

        assert (
            x.shape == x_transformed.shape
        ), "x has to have the same shape after transformation as before"
        assert (
            y.shape == y_transformed.shape
        ), "y has to have the same shape after transformation as before"

        return (x_transformed, y_transformed)

    return wrapper


@transfomer_decorator
def random_shuffle(x: ndarray, y: ndarray) -> Tuple[ndarray, ndarray]:
    # pylint: disable-msg=no-member
    permutation = np.random.RandomState(seed=SEED).permutation(x.shape[0])
    x_shuffled = x[permutation]
    y_shuffled = y[permutation]
    return x_shuffled, y_shuffled


@transfomer_decorator
def balanced_labels_shuffle(
    x: ndarray, y: ndarray, num_partitions=10
) -> Tuple[ndarray, ndarray]:
    """Shuffled y so that the labels are uniformly distributed in each section"""
    example_count = y.shape[0]
    section_size = int(example_count / num_partitions)

    assert (
        example_count % num_partitions == 0
    ), "Number of examples needs to be evenly divideable by section_count"

    x_shuffled, y_shuffled = random_shuffle(x, y)

    # Array of indices that sort a along the specified axis.
    sort_index = np.argsort(y_shuffled, axis=0)

    x_sorted = x_shuffled[sort_index]
    y_sorted = y_shuffled[sort_index]

    balance_index = (
        np.array(range(example_count), np.int64)
        .reshape((section_size, num_partitions))
        .transpose()
        .reshape(example_count)
    )

    x_balanced = x_sorted[balance_index]
    y_balanced = y_sorted[balance_index]

    return x_balanced, y_balanced


@transfomer_decorator
def group_by_label(x: ndarray, y: ndarray) -> Tuple[ndarray, ndarray]:
    """
    Shuffles y so that only a single label is in each section
    Number of sections will depend on number of unique labels
    """
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


@transfomer_decorator
def biased_balanced_labels_shuffle(  # pylint: disable=R0914
    x: ndarray, y: ndarray, bias=1000
) -> Tuple[ndarray, ndarray]:
    """
    Shuffle y so that the labels are uniformly distributed in each section
    except one label which will have a bias. Considering the bias the rest
    needs to be evenly dividable
    """
    example_count = y.shape[0]
    # section_count is equal to number of unique labels
    unique_labels_set = set(y)
    section_count = len(unique_labels_set)
    section_size = int(example_count / section_count)

    assert (
        example_count % section_count == 0
    ), "Number of examples needs to be evenly divideable by section_count"

    # Array of indices that sort a along the specified axis.
    sort_indexes = np.argsort(y, axis=0)

    x_sorted = x[sort_indexes]
    y_sorted = y[sort_indexes]

    x_splits, y_splits = split(x_sorted, y_sorted, num_splits=section_count)

    # Extract first "bias" from each split
    x_biased_splits = [x_split[:bias] for x_split in x_splits]
    y_biased_splits = [y_split[:bias] for y_split in y_splits]

    for y_biased_split in y_biased_splits:
        # Check that we got single label splits
        assert len(set(y_biased_split)) == 1

    # Merge rest
    x_unbiased = np.concatenate([x_split[bias:] for x_split in x_splits])
    y_unbiased = np.concatenate([y_split[bias:] for y_split in y_splits])

    assert x_unbiased.shape[0] == section_count * (
        section_size - bias
    ), "Length of unbiased elements should be equal to original length minus extracted bias"

    # Create balanced shuffle of rest
    x_balanced, y_balanced = balanced_labels_shuffle(
        x_unbiased, y_unbiased, num_partitions=section_count
    )

    for y_balanced_split in np.split(y_balanced, indices_or_sections=section_count):
        assert set(y_balanced_split) == unique_labels_set

    # split unbiased splits again to be merged with biased splits
    x_balanced_splits, y_balanced_splits = split(
        x_balanced, y_balanced, num_splits=section_count
    )

    x_merged = np.concatenate(
        [
            np.concatenate([x1, x2], axis=0)
            for x1, x2 in zip(x_biased_splits, x_balanced_splits)
        ]
    )
    y_merged = np.concatenate(
        [
            np.concatenate([y1, y2], axis=0)
            for y1, y2 in zip(y_biased_splits, y_balanced_splits)
        ]
    )

    assert x.shape == x_merged.shape, "Shape of x should not change"

    return x_merged, y_merged


@transfomer_decorator
def sorted_labels_sections_shuffle(  # pylint: disable=R0914
    x: ndarray, y: ndarray, num_partitions=100, class_per_partition=1
) -> Tuple[ndarray, ndarray]:
    """
    Does the following:
    1. Sort by label
    2. Shuffles sections randomley
    """
    assert (
        x.shape[0] % num_partitions == 0
    ), "Number of examples needs to be divisionable by num_partitions"

    num_examples = x.shape[0]
    num_examples_per_partition = num_examples // num_partitions

    # Array of indices that sort a along the specified axis.
    sort_indices = np.argsort(y, axis=0)

    # After sorting we will have num_labels sorted sections (e.g. 10 for MNIST)
    # e.g. with 4 labels and 8 examples (assuming each label occurs equal times)
    # => y = [0, 0, 1, 1, 2, 2, 3, 3]
    x_sorted = x[sort_indices]
    y_sorted = y[sort_indices]

    # Now we will init a permutation to shuffle our sorted examples
    permutation = np.array(range(num_examples), np.int64)

    num_sections = class_per_partition * num_partitions
    num_examples_per_section = num_examples_per_partition // class_per_partition
    permutation = permutation.reshape((num_sections, num_examples_per_section))

    # We will now create a random index which will shuffle the sections in
    # our permutation randomly on axis=0 before we later reshape the permutation
    # back into a list with length == num_examples and use it to shuffle our
    # x_sorted and y_sorted
    # pylint: disable-msg=no-member
    section_shuffle_indices = np.random.RandomState(seed=SEED).permutation(
        len(permutation)
    )

    permutation = permutation[section_shuffle_indices]
    permutation = permutation.reshape(num_examples)

    x_shuffled = x_sorted[permutation]
    y_shuffled = y_sorted[permutation]

    return (x_shuffled, y_shuffled)
