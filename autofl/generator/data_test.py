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


def test_shuffle():
    # Prepare
    x = np.array([1, 2, 3, 4])
    y = np.array([11, 12, 13, 14])
    # Execute
    xs, ys = data.shuffle(x, y, seed=42)
    # Assert
    for x, y in zip(xs, ys):
        assert x == (y - 10)
