import numpy as np
import pytest
import tensorflow as tf

from . import data

assert_equal = np.testing.assert_equal
assert_raises = np.testing.assert_raises


@pytest.mark.integration
def test_load():
    (x_train, y_train), (x_test, y_test) = data.load(tf.keras.datasets.mnist)
    assert x_train.shape[0] == y_train.shape[0]
    assert x_test.shape[0] == y_test.shape[0]
    assert len(x_train.shape) == len(x_test.shape)
    assert len(y_train.shape) == len(y_test.shape)


def test_split_num_splits_valid_max():
    # Prepare
    x = np.zeros((3, 28, 28))
    y = np.zeros((3))
    num_splits = 3
    # Execute
    x_splits, y_splits = data.split(x, y, num_splits)
    # Assert
    assert len(x_splits) == num_splits
    assert len(y_splits) == num_splits
    # By the transitive property of == also:
    # len(x_splits) == len(y_splits)


def test_split_num_splits_valid_min():
    # Prepare
    x = np.zeros((3, 28, 28))
    y = np.zeros((3))
    num_splits = 1
    # Execute
    x_splits, y_splits = data.split(x, y, num_splits)
    # Assert
    assert len(x_splits) == num_splits
    assert len(y_splits) == num_splits
    # By the transitive property of == also:
    # len(x_splits) == len(y_splits)


def test_split_num_splits_valid():
    # Prepare
    x = np.zeros((6, 28, 28))
    y = np.zeros((6))
    num_splits = 2
    # Execute
    x_splits, y_splits = data.split(x, y, num_splits)
    # Assert
    assert len(x_splits) == num_splits
    assert len(y_splits) == num_splits
    # By the transitive property of == also:
    # len(x_splits) == len(y_splits)


def test_split_num_splits_invalid():
    # Prepare
    x = np.zeros((3, 28, 28))
    y = np.zeros((3))
    num_splits = 2
    # Execute & assert
    try:
        _, _ = data.split(x, y, num_splits)
        pytest.fail()
    except ValueError:
        pass


def test_split_dims():
    # Prepare
    x = np.zeros((3, 28, 28))
    y = np.zeros((3))
    num_splits = 3
    # Execute
    x_splits, y_splits = data.split(x, y, num_splits)
    # Assert: Corresponding x and y have the same number of examples
    for xs, ys in zip(x_splits, y_splits):
        assert xs.shape[0] == ys.shape[0]

    # Assert: Each split has the same dimensionality (except for number of examples)
    assert all([xs.shape == x_splits[0].shape for i, xs in enumerate(x_splits)])
    assert all([ys.shape == y_splits[0].shape for i, ys in enumerate(y_splits)])


def test_extract_validation_set():
    # Prepare
    example_count = 1000
    validation_set_size = 100
    labels = list(range(0, 10))
    x = np.zeros((example_count, 28, 28), dtype=np.float64)
    y = np.tile(np.array(labels, dtype=np.int64), example_count // 10)

    # Shuffle to make sure that extract_validation_set
    # does not expect a sorted array in any form
    np.random.shuffle(y)

    # Execute
    (x_train, y_train), (x_val, y_val) = data.extract_validation_set(x, y, size=100)

    # Assert
    assert (
        x_val.shape[0] == y_val.shape[0] == validation_set_size
    ), "Validation set has wrong size"
    assert (
        x_train.shape[0] == y_train.shape[0] == example_count - validation_set_size
    ), "Train set has wrong size"

    assert (
        set(labels) == set(y_train) == set(y_val)
    ), "Train and validation set both need to contain all labels"

    label_counts_train = np.unique(y_train, return_counts=True)[1]
    label_counts_validation = np.unique(y_val, return_counts=True)[1]

    # Each label occurs equal times
    assert len(set(label_counts_train)) == len(set(label_counts_validation)) == 1


def test_random_shuffle():
    # Prepare
    x = np.array([1, 2, 3, 4])
    y = np.array([11, 12, 13, 14])
    # Execute
    xs, ys = data.random_shuffle(x, y)
    # Assert
    for x, y in zip(xs, ys):
        assert x == (y - 10)


def test_balanced_labels_shuffle_wrong_section_count():
    # Prepare
    examples = range(100, 200)
    sorted_labels = range(10)

    x = np.array(examples, dtype=np.int64)
    y = np.tile(np.array(sorted_labels, dtype=np.int64), 10)

    with pytest.raises(Exception):
        data.balanced_labels_shuffle(x, y, section_count=3)


@pytest.mark.parametrize(
    "section_count, example_count", [(2, 1000), (5, 1000), (10, 1000)]
)
def test_balanced_labels_shuffle(section_count, example_count):
    # Prepare
    unique_labels = range(10)  # 10 unique labels

    # Values will at the same time be their original labels
    # We will later use this for asserting if the label relationship is still present
    x = np.tile(
        np.array(unique_labels, dtype=np.int64), example_count // len(unique_labels)
    )

    # Shuffle to avoid any bias; been there...
    np.random.shuffle(x)

    y = np.copy(x)

    assert x.shape[0] == y.shape[0]

    # Execute
    x_balanced_shuffled, y_balanced_shuffled = data.balanced_labels_shuffle(
        x, y, section_count=section_count
    )

    # Assert
    # Create tuples for x,y splits so we can more easily analyze them
    x_splits = np.split(x_balanced_shuffled, indices_or_sections=section_count, axis=0)
    y_splits = np.split(y_balanced_shuffled, indices_or_sections=section_count, axis=0)

    # Check that each value still matches its label
    for (x_split, y_split) in zip(x_splits, y_splits):
        # Check that the split has the right size
        assert y_split.shape[0] == int(example_count / section_count)
        # Check that each segment contains each label
        assert set(y_split) == set(unique_labels)

        label_count_per_section = example_count / section_count / len(unique_labels)

        for c in np.unique(y_split, return_counts=True)[1]:
            assert c == label_count_per_section

        for x_i, y_i in zip(x_split, y_split):
            assert x_i == y_i


@pytest.mark.parametrize("bias, example_count", [(10, 1000), (20, 1000), (50, 1000)])
def test_biased_balanced_labels_shuffle(bias, example_count):  # pylint: disable=R0914
    # Prepare
    unique_labels_count = 10
    unique_labels = range(unique_labels_count)  # 10 unique labels
    section_size = example_count / unique_labels_count
    unbiased_label_count = (section_size - bias) / unique_labels_count

    # Values will at the same time be their original labels
    # We will later use this for asserting if the label relationship is still present
    x = np.tile(
        np.array(unique_labels, dtype=np.int64), example_count // len(unique_labels)
    )

    # Shuffle to avoid any bias; been there...
    np.random.shuffle(x)

    y = np.copy(x)

    assert x.shape[0] == y.shape[0]

    # Execute
    x_balanced_shuffled, y_balanced_shuffled = data.biased_balanced_labels_shuffle(
        x, y, bias=bias
    )

    # Assert
    # Create tuples for x,y splits so we can more easily analyze them
    x_splits = np.split(
        x_balanced_shuffled, indices_or_sections=unique_labels_count, axis=0
    )
    y_splits = np.split(
        y_balanced_shuffled, indices_or_sections=unique_labels_count, axis=0
    )

    # Check that each value still matches its label
    for split_index, xy_split in enumerate(zip(x_splits, y_splits)):
        x_split, y_split = xy_split

        # Check that the split has the right size
        assert y_split.shape[0] == int(example_count / unique_labels_count)
        # Check that each segment contains each label
        assert set(y_split) == set(
            unique_labels
        ), "Each label needs to be present in each section"

        unique_counts = np.unique(y_split, return_counts=True)[1]

        # The first split should contain a bias for the first label
        # The second split should contain a bias for the second label
        # repeat untill The "last label should..."
        for unique_counts_index, unique_count in enumerate(unique_counts):
            if split_index == unique_counts_index:
                assert (
                    unique_count == unbiased_label_count + bias
                ), "At split_index {} a bias should be present".format(split_index)
            else:
                assert unique_count == unbiased_label_count

        for x_i, y_i in zip(x_split, y_split):
            assert x_i == y_i


@pytest.mark.parametrize(
    "unique_labels_count, example_count", [(2, 1000), (5, 1000), (10, 1000)]
)
def test_group_by_label(unique_labels_count, example_count):
    # Prepare
    unique_labels = range(unique_labels_count)

    # Values will at the same time be their original labels
    # We will later use this for asserting if the label relationship is still present
    x = np.tile(
        np.array(unique_labels, dtype=np.int64), example_count // unique_labels_count
    )

    # Shuffle to avoid any bias; been there...
    np.random.shuffle(x)

    y = np.copy(x)

    assert x.shape[0] == y.shape[0]

    # Execute
    x_sectioned, y_sectioned = data.group_by_label(x, y)

    # Assert
    # Create tuples for x,y splits so we can more easily analyze them
    x_splits = np.split(x_sectioned, indices_or_sections=unique_labels_count, axis=0)
    y_splits = np.split(y_sectioned, indices_or_sections=unique_labels_count, axis=0)

    # Check that each value still matches its label
    for (x_split, y_split) in zip(x_splits, y_splits):
        # Check that the split has the right size
        assert y_split.shape[0] == int(example_count / unique_labels_count)

        # Check that each segment contains only one label
        assert len(set(y_split)) == 1

        # check that x,y is correctly matched
        for x_i, y_i in zip(x_split, y_split):
            assert x_i == y_i
