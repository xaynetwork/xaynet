import pytest

from ..conftest import create_mock_dataset
from ..types import FederatedDataset, KerasDataset
from . import data


def pytest_collection_modifyitems(items):
    for item in items:
        if not any(item.iter_markers()):
            item.add_marker("unmarked")


def create_empty_file(full_path):
    open(full_path, "a").close()


class MockKerasDataset:  # pylint: disable=too-few-public-methods
    """
    Used as a mock dataset which will go through the load method in the data.py module
    to make sure that the mock dataset stays compatible with the default load function
    for all datasets in the project
    """

    @staticmethod
    def load_data() -> KerasDataset:
        return create_mock_dataset()


@pytest.fixture
def mock_random_splits_2_dataset() -> FederatedDataset:
    """dataset mock after it went through internal load method"""
    return data.generate_splits(2, MockKerasDataset(), shuffle_train=False)
