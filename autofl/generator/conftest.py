import os
from typing import List

import pytest

from ..conftest import create_mock_dataset
from ..types import FederatedDataset, FnameNDArrayTuple, KerasDataset
from . import data, persistence


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
def mock_dataset() -> KerasDataset:
    """dataset mock after it went through internal load method"""
    return MockKerasDataset().load_data()


@pytest.fixture
def mock_random_splits_2_dataset() -> FederatedDataset:
    """dataset mock after it went through internal load method"""
    return data.generate_splits(2, MockKerasDataset(), shuffle_train=False)


@pytest.fixture
def mock_random_splits_1_dataset() -> FederatedDataset:
    """dataset mock after it went through internal load method"""
    return data.generate_splits(1, MockKerasDataset(), shuffle_train=False)


@pytest.fixture
def mock_random_splits_2_fname_ndarray_tuples() -> List[FnameNDArrayTuple]:
    dataset = data.generate_splits(2, MockKerasDataset(), shuffle_train=False)
    return persistence.dataset_to_fname_ndarray_tuple_list(dataset)


@pytest.fixture(scope="session")
def mock_datasets_dir(tmpdir_factory):
    dataset_dir = tmpdir_factory.mktemp("datasets")

    os.mkdir(dataset_dir.join("random_splits_2"))
    os.mkdir(dataset_dir.join("random_splits_10"))

    persistence.save_splits(
        dataset=data.generate_splits(2, MockKerasDataset()),
        storage_dir=str(dataset_dir.join("random_splits_2")),
    )

    persistence.save_splits(
        dataset=data.generate_splits(10, MockKerasDataset()),
        storage_dir=str(dataset_dir.join("random_splits_10")),
    )

    # Write two usually os generated files into the directories
    # to check if the loading methods can handle auto generated
    # os files
    create_empty_file(dataset_dir.join("random_splits_2/.DS_Store"))
    create_empty_file(dataset_dir.join("random_splits_10/.DS_Store"))

    return str(dataset_dir)
