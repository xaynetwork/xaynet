import numpy as np
import pytest
from absl import flags

from .types import KerasDataset

FLAGS = flags.FLAGS


def pytest_runtest_setup():
    # Invoking FLAGS will make the flags usable for the
    # test execution and avoid throwing an error
    FLAGS(
        argv=[
            "test",  # some app name required
            "--fetch_datasets=True",  # resetting to default at beginning of every test
        ]
    )


@pytest.fixture
def disable_fetch():
    FLAGS(["test", "--fetch_datasets=False"])


def create_mock_dataset() -> KerasDataset:
    labels = np.arange(10, dtype=np.int8)

    x_train = np.ones((600, 32, 32, 3), dtype=np.int8)
    y_train = np.tile(labels, 60)
    assert x_train.shape[0] == y_train.shape[0]

    x_test = np.ones((100, 32, 32, 3), dtype=np.int8)
    y_test = np.tile(labels, 10)
    assert x_test.shape[0] == y_test.shape[0]

    return (x_train, y_train), (x_test, y_test)


@pytest.fixture
def mock_dataset() -> KerasDataset:
    """dataset mock after it went through internal load method"""
    return create_mock_dataset()
