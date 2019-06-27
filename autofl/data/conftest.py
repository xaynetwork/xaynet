import os

import numpy as np
import pytest

from autofl.data import data, persistence, typing


class MockKerasDataset:  # pylint: disable=too-few-public-methods
    """
    Used as a mock dataset which will go through the load method in the data.py module
    to make sure that the mock dataset stays compatible with the default load function
    for all datasets in the project
    """

    @staticmethod
    def load_data() -> typing.KerasDataset:
        labels = np.arange(10, dtype=np.int8)

        x_train = np.ones((60, 32, 32, 3), dtype=np.int8)
        y_train = np.tile(labels, 6)

        x_test = np.ones((10, 32, 32, 3), dtype=np.int8)
        y_test = np.tile(labels, 1)

        return (x_train, y_train), (x_test, y_test)


@pytest.fixture
def mock_keras_dataset() -> MockKerasDataset:
    """keras dataset mock"""
    return MockKerasDataset()


@pytest.fixture
def mock_dataset() -> typing.Dataset:
    """dataset mock after it went through internal load method"""
    return data.load(MockKerasDataset())


@pytest.fixture
def mock_random_splits_10_dataset() -> typing.FederatedDataset:
    """dataset mock after it went through internal load method"""
    return data.load_splits(10, MockKerasDataset())


@pytest.fixture
def mock_random_splits_2_dataset() -> typing.FederatedDataset:
    """dataset mock after it went through internal load method"""
    return data.load_splits(2, MockKerasDataset())


@pytest.fixture
def mock_random_splits_1_dataset() -> typing.FederatedDataset:
    """dataset mock after it went through internal load method"""
    return data.load_splits(1, MockKerasDataset())


@pytest.fixture(scope="session")
def mock_datasets_dir(tmpdir_factory):
    dataset_dir = tmpdir_factory.mktemp("datasets")

    os.mkdir(dataset_dir.join("random_splits_2"))
    os.mkdir(dataset_dir.join("random_splits_10"))

    persistence.save_splits(
        filename_template="random_splits_2_{}.npy",
        dataset=data.load_splits(2, MockKerasDataset()),
        storage_dir=str(dataset_dir.join("random_splits_2")),
    )

    persistence.save_splits(
        filename_template="random_splits_10_{}.npy",
        dataset=data.load_splits(10, MockKerasDataset()),
        storage_dir=str(dataset_dir.join("random_splits_10")),
    )

    return str(dataset_dir)
