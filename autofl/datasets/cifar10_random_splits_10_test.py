from typing import Tuple

import numpy as np
import pytest

from . import cifar10_random_splits_10 as cifar10
from . import storage


@pytest.mark.slow
@pytest.mark.integration
def test_load_splits(tmp_path):
    # Execute
    xy_splits_actual, xy_test_actual = cifar10.load_splits(local_datasets_dir=tmp_path)

    # Assert
    assert isinstance(xy_splits_actual, list)
    assert isinstance(xy_test_actual, tuple)

    for xy in xy_splits_actual:
        x, y = xy

        assert isinstance(x, np.ndarray)
        assert isinstance(y, np.ndarray)


def test_load_split(monkeypatch, tmp_path):
    # Prepare
    split_id_expected = "05"
    split_hashes_expected = ("foo", "bar")
    x_expected = np.ones((3, 2))
    y_expected = np.ones((3))

    def mock_load_split(
        datasets_repository: str,
        dataset_name: str,
        split_id: str,
        split_hashes: Tuple[str, str],
        local_datasets_dir: str,
    ):
        # Assert: avoid linter warnings so checking some defaults
        assert isinstance(datasets_repository, str)
        assert dataset_name == cifar10.DATASET_NAME

        # Assert: Check if arguments ar passed through correctly
        assert split_id == split_id_expected
        assert split_hashes == split_hashes_expected
        assert local_datasets_dir == tmp_path

        return x_expected, y_expected

    # We are mocking the load_split method in the storage module
    # which will be called by the load_split method in the cifar10 module
    monkeypatch.setattr(storage, "load_split", mock_load_split)

    # Execute
    x_actual, y_actual = cifar10.load_split(
        split_id=split_id_expected,
        split_hashes=split_hashes_expected,
        local_datasets_dir=tmp_path,
    )

    np.testing.assert_equal(x_actual, x_expected)
    np.testing.assert_equal(y_actual, y_expected)
