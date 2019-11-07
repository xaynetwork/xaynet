import numpy as np
import pytest

from . import transformer


def test_random_shuffle():
    # Prepare
    x = np.array([1, 2, 3, 4])
    y = np.array([11, 12, 13, 14])
    # Execute
    xs, ys = transformer.random_shuffle(x, y)
    # Assert
    for x, y in zip(xs, ys):
        assert x == (y - 10)


def test_classes_balanced_randomized_per_partition_wrong_section_count():
    # Prepare
    examples = range(100, 200)
    sorted_labels = range(10)

    x = np.array(examples, dtype=np.int64)
    y = np.tile(np.array(sorted_labels, dtype=np.int64), 10)

    with pytest.raises(Exception):
        transformer.classes_balanced_randomized_per_partition(x, y, num_partitions=3)


@pytest.mark.parametrize(
    "section_count, example_count", [(2, 1000), (5, 1000), (10, 1000)]
)
def test_classes_balanced_randomized_per_partition(section_count, example_count):
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
    # pylint: disable=line-too-long
    x_balanced_shuffled, y_balanced_shuffled = transformer.classes_balanced_randomized_per_partition(
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
def test_one_biased_class_per_partition(bias, example_count):  # pylint: disable=R0914
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
    x_balanced_shuffled, y_balanced_shuffled = transformer.one_biased_class_per_partition(
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


@pytest.mark.parametrize(
    "example_count, num_partitions, cpp", [(44000, 100, 4), (54000, 100, 6)]
)
def test_class_per_partition(
    cpp, num_partitions, example_count
):  # pylint: disable=R0914
    # Prepare
    num_unique_classes = 10
    unique_labels = range(num_unique_classes)  # 10 unique labels

    x = np.ones((example_count, 28, 28), dtype=np.int64)

    y = np.tile(
        np.array(unique_labels, dtype=np.int64), example_count // num_unique_classes
    )

    # Assert that assumptions about input are correct
    assert x.shape[0] == example_count
    assert x.shape[0] == y.shape[0]

    # Execute
    x_shuffled, y_shuffled = transformer.class_per_partition(
        x, y, num_partitions=num_partitions, cpp=cpp
    )

    # Assert
    assert x.shape == x_shuffled.shape
    assert y.shape == y_shuffled.shape

    # Create tuples for x,y splits so we can more easily analyze them
    y_splits = np.split(y_shuffled, indices_or_sections=num_partitions, axis=0)

    actual_cpp = [len(set(y_split)) for y_split in y_splits]

    assert set(actual_cpp) == set([cpp])


@pytest.mark.slow
@pytest.mark.parametrize(
    "example_count, num_partitions, cpp",
    [
        (45000, 100, 1),
        (45000, 100, 6),
        (45000, 100, 10),
        (54000, 100, 1),
        (54000, 100, 4),
        (53900, 100, 7),
        (54000, 100, 10),
    ],
)
def test_class_per_partition_verbose(cpp, num_partitions, example_count):
    """Purpose of this test is to test even more verbose"""
    test_class_per_partition(cpp, num_partitions, example_count)


@pytest.mark.parametrize(
    "num_unique_classes, example_count", [(2, 1000), (5, 1000), (10, 1000)]
)
def test_sort_by_class(num_unique_classes, example_count):
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
    x_sectioned, y_sectioned = transformer.sort_by_class(x, y)

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
