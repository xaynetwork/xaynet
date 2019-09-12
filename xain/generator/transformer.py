from typing import Callable, Tuple

import numpy as np
from numpy import ndarray

from .class_per_partition_distribution import distribution as cpp_distribution

# Passed to RandomState for predictable shuffling
SEED = 851746


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
    # pylint: disable=no-member
    permutation = np.random.RandomState(seed=SEED).permutation(x.shape[0])
    x_shuffled = x[permutation]
    y_shuffled = y[permutation]
    return x_shuffled, y_shuffled


@transfomer_decorator
def classes_balanced_randomized_per_partition(
    x: ndarray, y: ndarray, num_partitions=10
) -> Tuple[ndarray, ndarray]:
    """Shuffles y so that only a each class is in each partition"""
    example_count = y.shape[0]
    section_size = int(example_count / num_partitions)

    assert (
        example_count % num_partitions == 0
    ), "Number of examples needs to be evenly divisible by section_count"

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
def sort_by_class(x: ndarray, y: ndarray) -> Tuple[ndarray, ndarray]:
    """
    Shuffles y so that only a single label is in each partition
    Number of partitions will depend on number of unique labels
    """
    example_count = y.shape[0]
    partition_count = np.unique(y).shape[0]

    assert (
        example_count % partition_count == 0
    ), "Number of examples needs to be evenly divisible by partition_count"

    # Array of indices that sort a along the specified axis.
    sort_indexes = np.argsort(y, axis=0)

    x_sorted = x[sort_indexes]
    y_sorted = y[sort_indexes]

    return x_sorted, y_sorted


@transfomer_decorator
def one_biased_class_per_partition(  # pylint: disable=R0914
    x: ndarray, y: ndarray, bias=1000
) -> Tuple[ndarray, ndarray]:
    """
    Shuffle y so that the labels are uniformly distributed in each section
    except one label which will have a bias. Considering the bias the rest
    needs to be evenly divisible
    """
    example_count = y.shape[0]
    # section_count is equal to number of unique labels
    unique_labels_set = set(y)
    section_count = len(unique_labels_set)
    section_size = int(example_count / section_count)

    assert (
        example_count % section_count == 0
    ), "Number of examples needs to be evenly divisible by section_count"

    # Array of indices that sort a along the specified axis.
    sort_indexes = np.argsort(y, axis=0)

    x_sorted = x[sort_indexes]
    y_sorted = y[sort_indexes]

    x_splits = np.split(x_sorted, indices_or_sections=section_count, axis=0)
    y_splits = np.split(y_sorted, indices_or_sections=section_count, axis=0)

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
    x_balanced, y_balanced = classes_balanced_randomized_per_partition(
        x_unbiased, y_unbiased, num_partitions=section_count
    )

    for y_balanced_split in np.split(y_balanced, indices_or_sections=section_count):
        assert set(y_balanced_split) == unique_labels_set

    # split unbiased splits again to be merged with biased splits
    x_balanced_splits = np.split(x_balanced, indices_or_sections=section_count, axis=0)
    y_balanced_splits = np.split(y_balanced, indices_or_sections=section_count, axis=0)

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
def class_per_partition(  # pylint: disable=R0914
    x: ndarray, y: ndarray, num_partitions: int, cpp: int
) -> Tuple[ndarray, ndarray]:
    """
    Does the following:
    1. Sort by label
    2. Shuffles sections randomley
    """
    assert x.shape[0] % num_partitions == 0, (
        f"Number of examples ({x.shape[0]}) needs to be divisible by "
        + "num_partitions ({num_partitions})"
    )

    num_classes = len(np.unique(y))
    num_sections = cpp * num_partitions

    assert num_sections % num_classes == 0, (
        f"number of sections ({num_sections}) needs to be divisible "
        + f"by number of classes ({num_classes})"
    )

    assert x.shape[0] % num_sections == 0, (
        f"number of examples ({x.shape[0]}) needs to be divisible "
        + f"by number of sections ({cpp * num_partitions})"
    )

    section_size = x.shape[0] // num_sections  # number of examples per section

    assert (x.shape[0] / num_classes) % section_size == 0, (
        f"number of examples per class ({x.shape[0] / num_classes}) needs to be divisible "
        + f"by number of examples per section ({section_size})"
    )

    # Array of indices that sort a along the specified axis.
    sort_indices = np.argsort(y, axis=0)

    # After sorting we will have num_labels sorted sections (e.g. 10 for MNIST)
    # e.g. with 4 labels and 8 examples (assuming each label occurs equal times)
    # => y = [0, 0, 1, 1, 2, 2, 3, 3]
    x_sorted = x[sort_indices]
    y_sorted = y[sort_indices]

    # We want to achive the following structure
    # global:      [ class 1 , ..., class N ]
    # per class:   [ section 1, ..., section N ]
    # per section: [ example 1, ..., example N ]
    new_x_shape = (num_classes, num_sections // num_classes, section_size, *x.shape[1:])
    new_y_shape = (num_classes, num_sections // num_classes, section_size, *y.shape[1:])

    x_sections = x_sorted.reshape(new_x_shape)
    y_sections = y_sorted.reshape(new_y_shape)

    # Type of dist is List[List[int]] with length num_partitions where each sublist
    # has length num_class and contains at each index a one if a class section should
    # occur in the final dataset partition
    cpp_dist = cpp_distribution(
        num_classes=num_classes, num_partitions=num_partitions, cpp=cpp
    )

    _, class_indices = np.nonzero(cpp_dist)
    section_indices = np.zeros((num_classes), dtype=np.int8)

    x_dist = []
    y_dist = []

    for c_idx in class_indices:
        s_idx = section_indices[c_idx]
        section_indices[c_idx] += 1

        x_sec = x_sections[c_idx][s_idx]
        y_sec = y_sections[c_idx][s_idx]

        x_dist.append(x_sec)
        y_dist.append(y_sec)

    x_dist = np.concatenate(x_dist)
    y_dist = np.concatenate(y_dist)

    return (x_dist, y_dist)
