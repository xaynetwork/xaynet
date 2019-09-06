from typing import Callable, List, Tuple

import numpy as np
from numpy import ndarray

from xain.types import FederatedDataset, KerasDataset

from .cpp_partition_distribution import cpp_partition_distribution

# Passed to RandomState for predictable shuffling
SEED = 851746


def load(keras_dataset) -> KerasDataset:
    (x_train, y_train), (x_test, y_test) = keras_dataset.load_data()

    y_train = y_train.reshape((y_train.shape[0],))
    y_test = y_test.reshape((y_test.shape[0],))

    return (x_train, y_train), (x_test, y_test)


def split(
    x: ndarray, y: ndarray, indices_or_sections: int
) -> Tuple[List[ndarray], List[ndarray]]:
    x_splits = np.split(x, indices_or_sections=indices_or_sections, axis=0)
    y_splits = np.split(y, indices_or_sections=indices_or_sections, axis=0)
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
    ), f"number of examples ({x.shape[0]}) needs to be evenly divisible by parameter size ({size})"

    assert size % len(set(y)) == 0, "size must be a multiple of number of labels"

    x_balanced, y_balanced = balanced_labels_shuffle(x, y)

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

    x, y = group_by_label(x, y)

    x_splits, y_splits = split(x, y, num_classes)

    x = np.concatenate([x_split[num_remove_per_class:] for x_split in x_splits])
    y = np.concatenate([y_split[num_remove_per_class:] for y_split in y_splits])

    return x, y


def generate_splits(
    num_partitions,
    keras_dataset,
    transformers,
    transformers_kwargs=None,
    validation_set_size=6000,
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

    x_splits, y_splits = split(x=x_train, y=y_train, indices_or_sections=num_partitions)

    assert y_splits

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
    # pylint: disable=no-member
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
def group_by_label(x: ndarray, y: ndarray) -> Tuple[ndarray, ndarray]:
    """
    Shuffles y so that only a single label is in each section
    Number of sections will depend on number of unique labels
    """
    example_count = y.shape[0]
    section_count = np.unique(y).shape[0]

    assert (
        example_count % section_count == 0
    ), "Number of examples needs to be evenly divisible by section_count"

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

    x_splits, y_splits = split(x_sorted, y_sorted, indices_or_sections=section_count)

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
        x_balanced, y_balanced, indices_or_sections=section_count
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
    # has length num_class and contains at each index the number of times the
    # class section should occur in the final dataset
    cs_dist = cpp_partition_distribution(
        num_classes=num_classes, num_partitions=num_partitions, cpp=cpp
    )

    _, class_indices = np.nonzero(cs_dist)
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
