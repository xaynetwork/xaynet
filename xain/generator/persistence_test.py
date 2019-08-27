import os

import numpy as np
import pytest

from xain.helpers import sha1

from ..types import FederatedDataset
from . import persistence


# Helper method to compare two federated datasets
def check_federated_dataset_equality(
    dataset_expected: FederatedDataset, dataset_actual: FederatedDataset
):
    xy_splits_expected, xy_val_expected, xy_test_expected = dataset_expected
    xy_splits_actual, xy_val_actual, xy_test_actual = dataset_actual

    assert len(xy_splits_expected) == len(xy_splits_actual)
    assert len(xy_test_expected) == len(xy_test_actual)

    # Assert
    for xy_expected, xy_actual in zip(xy_splits_expected, xy_splits_actual):
        x_expected, y_expected = xy_expected
        x_actual, y_actual = xy_actual

        np.testing.assert_equal(x_expected, x_actual)
        np.testing.assert_equal(y_expected, y_actual)

    assert xy_val_expected[0].shape == xy_val_actual[0].shape
    assert xy_val_expected[1].shape == xy_val_actual[1].shape

    assert xy_test_expected[0].shape == xy_test_actual[0].shape
    assert xy_test_expected[1].shape == xy_test_actual[1].shape


def test_dataset_to_fname_ndarray_tuple_list(mock_random_splits_2_dataset):
    # Prepare
    fnames_expected = [
        "x_00.npy",
        "y_00.npy",
        "x_01.npy",
        "y_01.npy",
        "x_val.npy",
        "y_val.npy",
        "x_test.npy",
        "y_test.npy",
    ]

    # Execute
    fname_ndarray_tuples = persistence.dataset_to_fname_ndarray_tuple_list(
        mock_random_splits_2_dataset
    )

    # Assert
    fnames_actual = [n for (n, _) in fname_ndarray_tuples]

    assert set(fnames_actual) == set(fnames_expected)

    for name, arr in fname_ndarray_tuples:
        assert isinstance(arr, np.ndarray)

        if "test" in name:
            assert arr.shape[0] == 100
        elif "val" in name:
            assert arr.shape[0] == 60
        else:
            assert arr.shape[0] == 270


def test_to_fname_ndarray_tuple():
    # Prepare
    x = np.ones((3, 2))
    y = np.ones((3))

    t_expected = [("x_00.npy", x), ("y_00.npy", y)]

    # Execute
    t_actual = persistence.to_fname_ndarray_tuple("00", (x, y))

    # Assert
    assert t_expected == t_actual


@pytest.mark.integration
def test_save(tmp_path):
    fname = "autofl_test_save_load_single.npy"
    fpath = os.path.join(tmp_path, fname)

    # Create NumPy array
    a_expected = np.zeros(shape=(3, 28, 28, 1), dtype=np.uint8)
    a_expected[0][1][1][0] = 255

    # Execute
    persistence.save(fname=fname, data=a_expected, storage_dir=tmp_path)

    # Assert
    a_actual = np.load(fpath)  # load ndarry

    np.testing.assert_equal(a_expected, a_actual)


def test_save_splits(monkeypatch, tmp_path, mock_random_splits_2_dataset):
    # Prepare
    dataset_name = "mock_dataset"
    xy_splits, xy_val, xy_test = mock_random_splits_2_dataset

    # -> Files which are supposed to be saved
    files_to_be_saved = [
        ("x_00.npy", xy_splits[0][0], tmp_path),
        ("y_00.npy", xy_splits[0][1], tmp_path),
        ("x_01.npy", xy_splits[1][0], tmp_path),
        ("y_01.npy", xy_splits[1][1], tmp_path),
        ("x_val.npy", xy_val[0], tmp_path),
        ("y_val.npy", xy_val[1], tmp_path),
        ("x_test.npy", xy_test[0], tmp_path),
        ("y_test.npy", xy_test[1], tmp_path),
    ]

    files_passed_to_save = []

    def mock_save(fname: str, data: np.ndarray, storage_dir: str):
        files_passed_to_save.append((fname, data, storage_dir))

    def mock_checksum(fpath: str):
        return "MOCK_CHECKSUM_FOR: {}".format(fpath)

    monkeypatch.setattr(persistence, "save", mock_save)
    monkeypatch.setattr(sha1, "checksum", mock_checksum)

    # Execute
    persistence.save_splits(
        dataset_name=dataset_name,
        dataset=mock_random_splits_2_dataset,
        local_generator_dir=tmp_path,
    )

    dataset_dir = os.path.join(tmp_path, dataset_name)

    # Assert
    for expected, actual in zip(files_to_be_saved, files_passed_to_save):
        assert expected[0] == actual[0]
        assert expected[1].shape == actual[1].shape
        assert dataset_dir == actual[2]


@pytest.mark.integration
def test_save_load_splits(tmp_path, mock_random_splits_2_dataset):
    # Prepare
    dataset_name = "mock_dataset"
    dataset_dir = os.path.join(tmp_path, dataset_name)

    def fpath(fname):
        return os.path.join(dataset_dir, fname)

    # Execute
    # Save splits into tmp directory
    persistence.save_splits(
        dataset_name=dataset_name,
        dataset=mock_random_splits_2_dataset,
        local_generator_dir=tmp_path,
    )

    # Assert
    # Load splits from tmp directory
    d = {
        # remove .npy ending with [:-4]
        fname[:-4]: np.load(fpath(fname))
        for fname in os.listdir(dataset_dir)
        if "npy" in fname
    }

    dataset_actual = (
        # train set
        [(d["x_00"], d["y_00"]), (d["x_01"], d["y_01"])],
        # validation set
        (d["x_val"], d["y_val"]),
        # test set
        (d["x_test"], d["y_test"]),
    )

    check_federated_dataset_equality(
        dataset_expected=mock_random_splits_2_dataset, dataset_actual=dataset_actual
    )
