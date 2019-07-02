import os

import numpy as np
import tensorflow as tf

import autofl.datasets.cifar10_random_splits_10 as ds
from autofl.data import persistence


def test_generate_dataset(mock_keras_dataset, monkeypatch):
    # Prepare
    monkeypatch.setattr(tf.keras.datasets, "cifar10", mock_keras_dataset)

    # Execute
    xy_splits, xy_test = ds.generate_dataset()

    # Assert
    # -> Verify types
    assert isinstance(xy_splits, list)
    assert isinstance(xy_test, tuple)
    assert len(xy_splits) == 10

    # -> Test shape of splits
    for split in xy_splits:
        x, y = split

        assert isinstance(x, np.ndarray)
        assert isinstance(y, np.ndarray)

        assert x.shape == (6, 32, 32, 3)
        assert y.shape == (6,)

    # -> Check test set
    x_test, y_test = xy_test

    assert x_test.shape == (10, 32, 32, 3)
    assert y_test.shape == (10,)


def test_load_split(monkeypatch, tmp_path):
    # Prepare
    split_index = 5
    xy_expected = (np.ones((3, 2)), np.ones((3)))
    x_expected, y_expected = xy_expected

    def mock_download_remote_ndarray(
        datasets_repository: str,
        dataset_name: str,
        split_name: str,
        local_datasets_dir: str,
    ):
        # Check if split_name contains the right index
        # Assert: Check if local_datasets_dir was correctly passed through
        assert local_datasets_dir == tmp_path

        if split_name == "x{}.npy".format(split_index):
            return x_expected
        if split_name == "y{}.npy".format(split_index):
            return y_expected

        raise Exception("split_name was incorrect")

    monkeypatch.setattr(
        persistence, "download_remote_ndarray", mock_download_remote_ndarray
    )

    # Execute
    x_actual, y_actual = ds.load_split(split_index, local_datasets_dir=tmp_path)

    # Assert
    np.testing.assert_equal(x_actual, x_actual)
    np.testing.assert_equal(y_actual, y_actual)


def test_load_test(monkeypatch, tmp_path):
    # Prepare
    xy_expected = (np.ones((3, 2)), np.ones((3)))
    x_expected, y_expected = xy_expected

    def mock_download_remote_ndarray(
        datasets_repository: str,
        dataset_name: str,
        split_name: str,
        local_datasets_dir: str,
    ):
        # Check if split_name contains the right index
        # Assert: Check if local_datasets_dir was correctly passed through
        assert local_datasets_dir == tmp_path

        if split_name == "x_test.npy":
            return x_expected
        if split_name == "y_test.npy":
            return y_expected

        raise Exception("split_name was incorrect")

    monkeypatch.setattr(
        persistence, "download_remote_ndarray", mock_download_remote_ndarray
    )

    # Execute
    x_actual, y_actual = ds.load_test(local_datasets_dir=tmp_path)

    # Assert
    np.testing.assert_equal(x_actual, x_actual)
    np.testing.assert_equal(y_actual, y_actual)
