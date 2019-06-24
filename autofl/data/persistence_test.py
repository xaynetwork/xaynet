import os

import numpy as np
import pytest

from autofl.data import persistence


def test_dataset_to_filename_ndarray_tuple(mock_cifar10_random_splits_2_dataset):
    # Prepare
    filenames_expected = [
        "my_template_x0.npy",
        "my_template_y0.npy",
        "my_template_x1.npy",
        "my_template_y1.npy",
        "my_template_x_test.npy",
        "my_template_y_test.npy",
    ]

    filename_tpl = "my_template_{}.npy"

    # Execute
    filename_ndarray_tuples = persistence.dataset_to_filename_ndarray_tuple(
        filename_tpl, mock_cifar10_random_splits_2_dataset
    )

    # Assert
    filenames = [n for (n, _) in filename_ndarray_tuples]

    assert set(filenames) == set(filenames_expected)

    for name, arr in filename_ndarray_tuples:
        assert isinstance(arr, np.ndarray)

        if "test" in name:
            assert arr.shape[0] == 10
        else:
            assert arr.shape[0] == 30


@pytest.mark.integration
def test_save_load_single(tmp_path):
    tmp_file = "autofl_test_save_load_single.npy"

    # Create NumPy array
    a_expected = np.zeros(shape=(3, 28, 28, 1), dtype=np.uint8)
    a_expected[0][1][1][0] = 255

    # Store to disk, then load from disk
    persistence.save(filename=tmp_file, data=a_expected, storage_dir=tmp_path)
    a_actual = persistence.load(filename=tmp_file, storage_dir=tmp_path)

    # Test equality
    assert np.array_equal(a_expected, a_actual)


@pytest.mark.integration
def test_save_load_multi(tmp_path):
    # Prepare
    tmp_file = "autofl_test_save_load_multi.npy"

    # -> Create NumPy array
    x0 = np.ones(shape=(3, 3, 3), dtype=np.uint8)
    x1 = np.ones(shape=(3, 3, 3), dtype=np.uint8)
    x2 = np.ones(shape=(3, 3, 3), dtype=np.uint8)

    x_all = np.array([x0, x1, x2])

    # -> Store to disk, then load from disk
    persistence.save(filename=tmp_file, data=x_all, storage_dir=tmp_path)

    # Execute
    x_all_actual = persistence.load(filename=tmp_file, storage_dir=tmp_path)

    # Assert
    # -> Test equality
    assert np.array_equal(x_all, x_all_actual)

    # -> Test equality of original arrays
    x0_ex, x1_ex, x2_ex = np.array([x0, x1, x2])

    assert np.array_equal(x0, x0_ex)
    assert np.array_equal(x1, x1_ex)
    assert np.array_equal(x2, x2_ex)


@pytest.mark.integration
def test_save_splits_with_save(mock_cifar10_random_splits_1_dataset, tmp_path):
    # Prepare
    # -> Files which are supposed to be created in tmp_path
    files_to_be_created = [
        "tpl_x0.npy",
        "tpl_y0.npy",
        "tpl_y_test.npy",
        "tpl_x_test.npy",
    ]

    # -> Using mock_cifar10_random_splits_1_dataset
    xy_splits, xy_test = mock_cifar10_random_splits_1_dataset

    # Execute
    persistence.save_splits(
        # filename_template is not relevant for the
        # test as the mock will ignore it
        filename_template="tpl_{}.npy",
        dataset=mock_cifar10_random_splits_1_dataset,
        storage_dir=tmp_path,
    )

    # Assert
    files_in_storage_dir = os.listdir(tmp_path)

    assert set(files_in_storage_dir) == set(files_to_be_created)

    data = {
        filename: np.load("{}/{}".format(tmp_path, filename))
        for filename in files_to_be_created
    }

    assert data["tpl_x0.npy"].shape == xy_splits[0][0].shape
    assert data["tpl_y0.npy"].shape == xy_splits[0][1].shape
    assert data["tpl_x_test.npy"].shape == xy_test[0].shape
    assert data["tpl_y_test.npy"].shape == xy_test[1].shape
