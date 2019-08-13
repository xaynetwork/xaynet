import numpy as np
import pytest

from ..conftest import create_mock_keras_dataset
from ..types import FederatedDataset, KerasDataset
from . import data


class MockKerasDataset:  # pylint: disable=too-few-public-methods
    """
    Used as a mock dataset which will go through the load method in the data.py module
    to make sure that the mock dataset stays compatible with the default load function
    for all datasets in the project
    """

    @staticmethod
    def load_data() -> KerasDataset:
        return create_mock_keras_dataset()


def no_shuffle(x: np.ndarray, y: np.ndarray):
    return x, y


@pytest.fixture
def mock_random_splits_2_dataset() -> FederatedDataset:
    """dataset mock after it went through internal load method"""
    return data.generate_splits(
        num_splits=2,
        validation_set_size=60,
        keras_dataset=MockKerasDataset(),
        transformer=no_shuffle,
    )


@pytest.fixture
def mock_simple_keras_dataset():
    # train set with numbers 0, 7, 1, 2 as 3x3 images/matrixes
    x_train = np.array(
        [
            [[1, 1, 1], [1, 0, 1], [1, 1, 1]],
            [[1, 1, 1], [0, 1, 0], [1, 0, 0]],
            [[0, 1, 0], [0, 1, 0], [0, 1, 0]],
            [[1, 1, 1], [0, 1, 0], [1, 1, 1]],
        ],
        dtype=np.int8,
    )
    y_train = np.array([0, 7, 1, 2], dtype=np.int8)

    # test set with number 1 as matrix
    x_test = np.array([[[0, 0, 1], [0, 0, 1], [0, 0, 1]]], dtype=np.int8)
    y_test = np.array([1], dtype=np.int8)

    assert x_train.shape[0] == y_train.shape[0]
    assert x_test.shape[0] == y_test.shape[0]

    assert x_train.shape[1] == x_test.shape[1]
    assert x_train.shape[2] == x_test.shape[2]

    return (x_train, y_train), (x_test, y_test)


@pytest.fixture
def mock_simple_federated_dataset():
    # train set with numbers 0, 7, 1, 2 as 3x3 images/matrixes
    x_train = np.array(
        [
            [[1, 1, 1], [1, 0, 1], [1, 1, 1]],
            [[0, 1, 0], [0, 1, 0], [0, 1, 0]],
            [[1, 1, 1], [0, 1, 0], [1, 0, 0]],
        ],
        dtype=np.int8,
    )
    y_train = np.array([0, 1, 7], dtype=np.int8)

    x_val = np.array([[[1, 1, 1], [0, 1, 0], [1, 1, 1]]], dtype=np.int8)
    y_val = np.array([2], dtype=np.int8)

    # test set with number 1 as matrix
    x_test = np.array([[[0, 0, 1], [0, 0, 1], [0, 0, 1]]], dtype=np.int8)
    y_test = np.array([1], dtype=np.int8)

    assert x_train.shape[0] == y_train.shape[0]
    assert x_val.shape[0] == y_val.shape[0]
    assert x_test.shape[0] == y_test.shape[0]

    x_splits = np.split(x_train, indices_or_sections=3, axis=0)
    y_splits = np.split(y_train, indices_or_sections=3, axis=0)

    return zip(x_splits, y_splits), (x_val, y_val), (x_test, y_test)
