import numpy as np
import pytest

from .types import KerasDataset


def create_mock_dataset() -> KerasDataset:
    labels = np.arange(10, dtype=np.int8)

    x_train = np.ones((60, 32, 32, 3), dtype=np.int8)
    y_train = np.tile(labels, 6)

    x_test = np.ones((10, 32, 32, 3), dtype=np.int8)
    y_test = np.tile(labels, 1)

    return (x_train, y_train), (x_test, y_test)


@pytest.fixture
def mock_dataset() -> KerasDataset:
    """dataset mock after it went through internal load method"""
    return create_mock_dataset()
