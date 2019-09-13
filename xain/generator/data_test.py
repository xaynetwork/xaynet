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
