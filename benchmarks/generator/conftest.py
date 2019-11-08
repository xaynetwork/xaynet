import numpy as np
import pytest

from benchmarks.conftest import create_mock_keras_dataset
from xain.types import FederatedDataset, KerasDataset

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
    return data.create_federated_dataset(
        keras_dataset=MockKerasDataset(),
        num_partitions=2,
        validation_set_size=60,
        transformers=[no_shuffle],
    )
