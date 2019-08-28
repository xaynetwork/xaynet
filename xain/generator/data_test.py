import numpy as np
import pytest
import tensorflow as tf

from . import data


@pytest.mark.integration
def test_load():
    (x_train, y_train), (x_test, y_test) = data.load(tf.keras.datasets.mnist)
    assert x_train.shape[0] == y_train.shape[0]
    assert x_test.shape[0] == y_test.shape[0]
    assert len(x_train.shape) == len(x_test.shape)
    assert len(y_train.shape) == len(y_test.shape)


def test_remove_balanced(mock_keras_dataset):
    # Prepare
    (x, y), _ = mock_keras_dataset
    num_examples = x.shape[0]
    num_to_be_removed = 100
    [unique_classes, num_classes_per_class] = np.unique(y, return_counts=True)
    expected_num_classes_per_class = [
        n - num_to_be_removed // len(unique_classes) for n in num_classes_per_class
    ]

    # Execute
    x, y = data.remove_balanced(x=x, y=y, num_remove=num_to_be_removed)

    # Assert
    assert isinstance(x, np.ndarray)
    assert isinstance(y, np.ndarray)

    assert x.shape[0] == y.shape[0] == num_examples - num_to_be_removed

    [_, actual_num_classes_per_class] = np.unique(y, return_counts=True)

    # Each class should occur equal times so we use
    # the `set` method which makes comparision easier
    assert set(expected_num_classes_per_class) == set(actual_num_classes_per_class)


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
        data.balanced_labels_shuffle(x, y, num_partitions=3)


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
        x, y, num_partitions=section_count
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
    num_unique_classes = 10
    unique_labels = range(num_unique_classes)  # 10 unique labels
    section_size = example_count // num_unique_classes
    # Amount of labels per partition without any bias
    unbiased_label_count = (section_size - bias) / num_unique_classes

    x = np.ones((example_count, 28, 28), dtype=np.int64)
    y = np.tile(np.array(unique_labels, dtype=np.int64), section_size)

    assert x.shape[0] == y.shape[0]

    # Execute
    x_balanced_shuffled, y_balanced_shuffled = data.biased_balanced_labels_shuffle(
        x, y, bias=bias
    )

    # Assert
    # Create tuples for x,y splits so we can more easily analyze them
    x_splits = np.split(
        x_balanced_shuffled, indices_or_sections=num_unique_classes, axis=0
    )
    y_splits = np.split(
        y_balanced_shuffled, indices_or_sections=num_unique_classes, axis=0
    )

    # Check that each value still matches its label
    for split_index, xy_split in enumerate(zip(x_splits, y_splits)):
        _, y_split = xy_split

        # Check that the split has the right size
        assert y_split.shape[0] == int(example_count / num_unique_classes)
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


# def test_Bucket(replace, num_classes, num_per_class, num_per_pick):
#     # Prepare
#     bucket = data.Bucket(num_classes=num_classes, num_per_class=num_per_class)
#     expected_bucket_size = num_classes * num_per_class
#     expected_total_picks_till_empty = math.ceil(expected_bucket_size / num_per_pick)

#     # Execute
#     if not replace and num_per_pick > num_classes:
#         with pytest.raises(Exception):
#             bucket.pick(num_per_pick, replace=replace)
#             # FIXME: don't rturn rather use some pytest method here
#         return

#     picks = np.array([], dtype=np.int8)

#     # try to pick too much
#     for _ in range(expected_total_picks_till_empty):
#         picks = np.concatenate([picks, bucket.pick(num_per_pick, replace=replace)])

#     # Assert
#     # picks should contain num_classes * num_per_class entries
#     assert picks.size == expected_bucket_size

#     # Bucket should be empty after num_per_class picks
#     assert set(bucket.storage) == set([0])

#     actual_unq_classes = np.unique(picks, return_counts=True)[1]

#     # Each class should be picked equal amount times
#     assert len(set(actual_unq_classes)) == 1


@pytest.mark.parametrize("cpp", [1, 5, 10])
def test_partition_distribution(cpp):
    # Prepare
    num_partitions = 10
    num_classes = 10
    num_sections = cpp * num_partitions
    num_per_class = num_sections / num_classes

    # Execute
    p_dist = data.partition_distribution(
        num_classes=num_classes, num_partitions=num_partitions, cpp=cpp
    )

    # Assert
    assert len(p_dist) == num_partitions

    for partition in p_dist:
        non_zero = np.count_nonzero(partition)
        assert non_zero == cpp

    assert np.sum(p_dist) == num_sections

    # Each class should occur globally same number of times
    for column in p_dist.T:
        assert np.sum(column) == num_per_class


@pytest.mark.parametrize("cpp", [1, 5, 10])
@pytest.mark.parametrize("example_count, num_partitions", [(400, 20), (1000, 100)])
def test_sorted_labels_sections_shuffle(
    cpp, num_partitions, example_count
):  # pylint: disable=R0914
    # Prepare
    num_unique_classes = 10
    unique_labels = range(num_unique_classes)  # 10 unique labels
    section_size = int(example_count / num_partitions)

    # Assert that assumptions about input are correct
    assert example_count % num_partitions == 0
    assert example_count % (2 * num_partitions) == 0
    assert section_size % num_unique_classes == 0

    x = np.ones((example_count, 28, 28), dtype=np.int64)

    y = np.tile(
        np.array(unique_labels, dtype=np.int64), example_count // num_unique_classes
    )

    # Assert that assumptions about input are correct
    assert x.shape[0] == example_count
    assert x.shape[0] == y.shape[0]

    # Execute
    x_shuffled, y_shuffled = data.sorted_labels_sections_shuffle(
        x, y, num_partitions=num_partitions, cpp=cpp
    )

    # Assert
    assert x.shape == x_shuffled.shape
    assert y.shape == y_shuffled.shape

    # Create tuples for x,y splits so we can more easily analyze them
    y_splits = np.split(y_shuffled, indices_or_sections=num_partitions, axis=0)

    actual_cpp = [len(set(y_split)) for y_split in y_splits]

    assert set(actual_cpp) == set([cpp])


@pytest.mark.parametrize(
    "num_unique_classes, example_count", [(2, 1000), (5, 1000), (10, 1000)]
)
def test_group_by_label(num_unique_classes, example_count):
    # Prepare
    unique_labels = range(num_unique_classes)

    # Values will at the same time be their original labels
    # We will later use this for asserting if the label relationship is still present
    x = np.tile(
        np.array(unique_labels, dtype=np.int64), example_count // num_unique_classes
    )

    # Shuffle to avoid any bias; been there...
    np.random.shuffle(x)

    y = np.copy(x)

    assert x.shape[0] == y.shape[0]

    # Execute
    x_sectioned, y_sectioned = data.group_by_label(x, y)

    # Assert
    # Create tuples for x,y splits so we can more easily analyze them
    x_splits = np.split(x_sectioned, indices_or_sections=num_unique_classes, axis=0)
    y_splits = np.split(y_sectioned, indices_or_sections=num_unique_classes, axis=0)

    # Check that each value still matches its label
    for (x_split, y_split) in zip(x_splits, y_splits):
        # Check that the split has the right size
        assert y_split.shape[0] == int(example_count / num_unique_classes)

        # Check that each segment contains only one label
        assert len(set(y_split)) == 1

        # check that x,y is correctly matched
        for x_i, y_i in zip(x_split, y_split):
            assert x_i == y_i
