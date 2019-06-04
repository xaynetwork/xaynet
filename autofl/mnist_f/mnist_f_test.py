import numpy as np
from autofl.mnist_f import main


def test_load():
    x_train, y_train, x_test, y_test = main.load()
    assert x_train.shape[0] == y_train.shape[0]
    assert x_test.shape[0] == y_test.shape[0]
    assert len(x_train.shape) == len(x_test.shape)
    assert len(y_train.shape) == len(y_test.shape)


def test_split_num_splits():
    # Prepare
    x = np.zeros((3, 28, 28))
    y = np.zeros((3))
    num_splits = 3
    # Execute
    x_splits, y_splits = main.split(x, y, num_splits)
    # Assert
    assert len(x_splits) == num_splits
    assert len(y_splits) == num_splits
    # By the transitive property of == also:
    # len(x_splits) == len(y_splits)


def test_split_dims():
    # Prepare
    x = np.zeros((3, 28, 28))
    y = np.zeros((3))
    num_splits = 3
    # Execute
    x_splits, y_splits = main.split(x, y, num_splits)
    # Assert: Corresponding x and y have the same number of examples
    for xs, ys in zip(x_splits, y_splits):
        assert xs.shape[0] == ys.shape[0]
    # TODO Assert: Each split has the same dimensionality (except for number of examples)


def test_shuffle():
    # Prepare
    x = np.array([1, 2, 3, 4])
    y = np.array([11, 12, 13, 14])
    # Execute
    xs, ys = main.shuffle(x, y, seed=42)
    # Assert
    for x, y in zip(xs, ys):
        assert x == (y - 10)
