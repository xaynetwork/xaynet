import numpy as np
import pytest
from absl import flags

from xain_fl.types import FederatedDataset, KerasDataset

FLAGS = flags.FLAGS


def pytest_collection_modifyitems(items):
    for item in items:
        if not any(item.iter_markers()):
            item.add_marker("unmarked")


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
def output_dir(tmpdir):
    tmpdir = str(tmpdir)
    FLAGS(["test", f"--output_dir={tmpdir}"])

    return tmpdir


@pytest.fixture
def disable_fetch():
    FLAGS(["test", "--fetch_datasets=False"])


def create_mock_keras_dataset() -> KerasDataset:
    labels = np.arange(10, dtype=np.int8)

    x_train = np.ones((600, 32, 32, 3), dtype=np.int8)
    y_train = np.tile(labels, 60)
    assert x_train.shape[0] == y_train.shape[0]

    x_test = np.ones((100, 32, 32, 3), dtype=np.int8)
    y_test = np.tile(labels, 10)
    assert x_test.shape[0] == y_test.shape[0]

    return (x_train, y_train), (x_test, y_test)


def create_mock_federated_dataset() -> FederatedDataset:
    labels = np.arange(10, dtype=np.int8)

    x_train = np.ones((540, 32, 32, 3), dtype=np.int8)
    y_train = np.tile(labels, 54)
    assert x_train.shape[0] == y_train.shape[0]

    xy_partitions = list(
        zip(
            np.split(x_train, indices_or_sections=2, axis=0),
            np.split(y_train, indices_or_sections=2, axis=0),
        )
    )

    x_val = np.ones((60, 32, 32, 3), dtype=np.int8)
    y_val = np.tile(labels, 6)
    assert x_val.shape[0] == y_val.shape[0]

    x_test = np.ones((100, 32, 32, 3), dtype=np.int8)
    y_test = np.tile(labels, 10)
    assert x_test.shape[0] == y_test.shape[0]

    return xy_partitions, (x_val, y_val), (x_test, y_test)


@pytest.fixture
def mock_keras_dataset() -> KerasDataset:
    """dataset mock after it went through internal load method"""
    return create_mock_keras_dataset()


@pytest.fixture
def mock_federated_dataset() -> FederatedDataset:
    """dataset mock after it went through internal load method"""
    return create_mock_federated_dataset()
