from typing import Callable, List, Tuple

import numpy as np
from numpy import ndarray

from xain.types import FederatedDataset, KerasDataset

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
    num_partitions: int,
    keras_dataset,
    transformers,
    transformers_kwargs=None,
    validation_set_size=6000,
) -> FederatedDataset:
    (x_train, y_train), xy_test = load(keras_dataset)

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

    x_splits, y_splits = split(x_train, y_train, num_partitions)

    xy_splits = list(zip(x_splits, y_splits))

    return xy_splits, xy_val, xy_test


class Bucket:
    def __init__(self, num_classes: int, num_per_class: int, dtype=np.int8):
        self.dtype = dtype
        self.num_class = num_classes

        # Let the bucket have its own RandomState although beware that each  bucket will have it
        # initialized the same. Given a sequence of method calls the result will always be the same
        # pylint: disable=no-member
        self.rst = np.random.RandomState(seed=SEED)

        # index == class and value == how many are left
        self.storage = np.full((num_classes), num_per_class, dtype=self.dtype)

    def zero_indicies(self):
        return np.where(self.storage == 0)[0]

    def multi_indicies(self):
        """Returns indices which have more than one section left"""
        return np.where(self.storage > 1)[0]

    def has_distinct_sections(self, num_distinct_sections: int) -> bool:
        possible_choices = np.flatnonzero(self.storage)
        return possible_choices.size >= num_distinct_sections

    def sample(self, num_distinct_sections: int):
        possible_choices = np.flatnonzero(self.storage)
        choices = self.rst.choice(
            possible_choices, num_distinct_sections, replace=False
        )
        # pylint: disable=no-member
        np.subtract.at(self.storage, choices, 1)

        return choices

    def inc_dec(self, index_inc: int, index_dec: int) -> None:
        # pylint: disable=no-member
        np.subtract.at(self.storage, [index_dec], 1)
        np.add.at(self.storage, [index_inc], 1)


def partition_distribution(
    num_classes: int, num_partitions: int, cpp: int
) -> np.ndarray:
    """
    :param num_classes: number of distinct unique classes
    :param num_partitions: number of partitions
    :param cpp: number of classes per partition required

    :returns: parition distribution as an ndarray of shape (num_partitions, num_classes)
              with ones at the locations where a section should be
    """
    # pylint: disable=no-member
    rst = np.random.RandomState(seed=SEED)

    dtype = np.int8
    num_sections = cpp * num_partitions

    assert num_sections % num_classes == 0
    num_per_class = num_sections // num_classes

    partitions = np.zeros((num_partitions, num_classes), dtype=dtype)

    # e.g. [20, 20, 20, 20] for num_classes=4 and cpp = 2
    bucket = Bucket(num_classes, num_per_class)

    for p in partitions:
        # 1. Check if there are 5 distinct non-zero values in the bucket
        while not bucket.has_distinct_sections(cpp):
            # Swap a section (which is available multiple times) from the bucket
            # with a section of a partition where that partition does not contain
            # the bucket section yet

            # pull from partition and fill bucket
            bucket_zero_indicies = bucket.zero_indicies()

            # pull from bucket and fill partition
            bucket_multi_indicies = bucket.multi_indicies()

            # sample one index from each
            bucket_zero_index = rst.choice(bucket_zero_indicies, 1)[0]
            bucket_multi_index = rst.choice(bucket_multi_indicies, 1)[0]

            # Find partition where:
            # - index_zero is at least one
            # - index_multi is zero
            partition_candidates_zero = np.where(partitions.T[bucket_zero_index] == 1)
            partition_candidates_multi = np.where(partitions.T[bucket_multi_index] == 0)
            partition_candidates = np.intersect1d(
                partition_candidates_zero, partition_candidates_multi
            )

            # Pick one partiton to modify
            pc_index = rst.choice(partition_candidates, 1)[0]

            # Swap
            partitions[pc_index][bucket_multi_index] += 1
            partitions[pc_index][bucket_zero_index] -= 1
            bucket.inc_dec(index_inc=bucket_zero_index, index_dec=bucket_multi_index)

        # 2. Sample from the bucket
        p_indicies = bucket.sample(num_distinct_sections=cpp)
        np.add.at(p, p_indicies, 1)

    return partitions


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
    cs_dist = partition_distribution(
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
