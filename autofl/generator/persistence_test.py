import os

import numpy as np
import pytest

from ..types import FederatedDataset
from . import persistence


# Helper method to compare two federated datasets
def check_federated_dataset_equality(
    dataset_expected: FederatedDataset, dataset_actual: FederatedDataset
):
    xy_splits_expected, xy_test_expected = dataset_expected
    xy_splits_actual, xy_test_actual = dataset_actual

    assert len(xy_splits_expected) == len(xy_splits_actual)
    assert len(xy_test_expected) == len(xy_test_actual)

    # Assert
    for xy_expected, xy_actual in zip(xy_splits_expected, xy_splits_actual):
        x_expected, y_expected = xy_expected
        x_actual, y_actual = xy_actual

        np.testing.assert_equal(x_expected, x_actual)
        np.testing.assert_equal(y_expected, y_actual)

    assert xy_test_expected[0].shape == xy_test_actual[0].shape
    assert xy_test_expected[1].shape == xy_test_actual[1].shape


def test_dataset_to_fname_ndarray_tuple_list(mock_random_splits_2_dataset):
    # Prepare
    fnames_expected = [
        "x0.npy",
        "y0.npy",
        "x1.npy",
        "y1.npy",
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
            assert arr.shape[0] == 10
        else:
            assert arr.shape[0] == 30


def test_to_fname_ndarray_tuple():
    # Prepare
    x = np.ones((3, 2))
    y = np.ones((3))

    t_expected = [("x0.npy", x), ("y0.npy", y)]

    # Execute
    t_actual = persistence.to_fname_ndarray_tuple("0", (x, y))

    # Assert
    assert t_expected == t_actual


@pytest.mark.integration
def test_save_load_single(tmp_path):
    tmp_file = "autofl_test_save_load_single.npy"

    # Create NumPy array
    a_expected = np.zeros(shape=(3, 28, 28, 1), dtype=np.uint8)
    a_expected[0][1][1][0] = 255

    # Store to disk, then load from disk
    persistence.save(fname=tmp_file, data=a_expected, storage_dir=tmp_path)
    a_actual = persistence.load(fname=tmp_file, storage_dir=tmp_path)

    # Test equality
    np.testing.assert_equal(a_expected, a_actual)


def test_save_splits(monkeypatch, tmp_path, mock_random_splits_1_dataset):
    # Prepare
    # -> Using mock_random_splits_1_dataset
    xy_splits, xy_test = mock_random_splits_1_dataset

    # -> Files which are supposed to be saved
    files_to_be_saved = [
        ("x0.npy", xy_splits[0][0], tmp_path),
        ("y0.npy", xy_splits[0][1], tmp_path),
        ("x_test.npy", xy_test[0], tmp_path),
        ("y_test.npy", xy_test[1], tmp_path),
    ]

    files_passed_to_save = []

    def mock_save(fname: str, data: np.ndarray, storage_dir: str):
        files_passed_to_save.append((fname, data, storage_dir))

    monkeypatch.setattr(persistence, "save", mock_save)

    # Execute
    persistence.save_splits(dataset=mock_random_splits_1_dataset, storage_dir=tmp_path)

    # Assert
    for expected, actual in zip(files_to_be_saved, files_passed_to_save):
        assert expected[0] == actual[0]
        assert expected[1].shape == actual[1].shape
        assert expected[2] == actual[2]


@pytest.mark.integration
def test_list_files_for_dataset(mock_datasets_dir):
    """
    Check if we can list files from given directory correctly
    """
    # Prepare
    fnames_expected = [
        "x0.npy",
        "y0.npy",
        "x1.npy",
        "y1.npy",
        "x_test.npy",
        "y_test.npy",
    ]

    dataset_dir = os.path.join(mock_datasets_dir, "random_splits_2")

    # Execute
    fnames_actual = persistence.list_files_for_dataset(storage_dir=dataset_dir)

    # Assert
    assert set(fnames_expected) == set(fnames_actual)


def test_dataset_from_fname_ndarray_tuples(
    mock_random_splits_2_dataset, mock_random_splits_2_fname_ndarray_tuples
):
    # Execute
    dataset_actual = persistence.dataset_from_fname_ndarray_tuples(
        mock_random_splits_2_fname_ndarray_tuples
    )

    check_federated_dataset_equality(
        dataset_expected=mock_random_splits_2_dataset, dataset_actual=dataset_actual
    )


@pytest.mark.integration
def test_save_load_splits(tmp_path, mock_random_splits_2_dataset):
    # Execute
    # Save splits into tmp directory
    persistence.save_splits(dataset=mock_random_splits_2_dataset, storage_dir=tmp_path)

    # Load splits from tmp directory
    dataset_actual = persistence.load_splits(storage_dir=tmp_path)

    # Assert
    check_federated_dataset_equality(
        dataset_expected=mock_random_splits_2_dataset, dataset_actual=dataset_actual
    )


@pytest.mark.integration
def test_list_datasets(mock_datasets_dir):
    # Prepare
    expected_datasets = set(["random_splits_2", "random_splits_10"])

    # Execute
    actual_datasets = persistence.list_datasets(local_generator_dir=mock_datasets_dir)

    # Assert
    assert expected_datasets == actual_datasets


def test_load_local_dataset(monkeypatch, tmp_path, mock_random_splits_2_dataset):
    # Prepare
    dataset_name = "my_dataset"
    dataset_expected = mock_random_splits_2_dataset

    def mock_list_datasets(local_generator_dir: str):
        # Assert: Check if list_datasets receives the correct arguments
        assert local_generator_dir == tmp_path
        return set([dataset_name])

    def mock_load_splits(storage_dir: str):
        # Assert: Check if load_splits receives the correct arguments
        dataset_dir = os.path.join(tmp_path, dataset_name)

        assert storage_dir == dataset_dir
        return dataset_expected

    monkeypatch.setattr(persistence, "list_datasets", mock_list_datasets)
    monkeypatch.setattr(persistence, "load_splits", mock_load_splits)

    # Execute
    dataset_actual = persistence.load_local_dataset(
        dataset_name=dataset_name, local_generator_dir=tmp_path
    )

    # Assert
    check_federated_dataset_equality(dataset_expected, dataset_actual)
