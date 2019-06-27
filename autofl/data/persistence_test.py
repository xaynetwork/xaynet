import numpy as np
import pytest

from autofl.data import config, persistence


def test_dataset_to_filename_ndarray_tuple(mock_random_splits_2_dataset):
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
        filename_tpl, mock_random_splits_2_dataset
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


def test_save_splits(monkeypatch, tmp_path, mock_random_splits_1_dataset):
    # Prepare
    filename_template = "tpl_{}.npy"

    # -> Using mock_random_splits_1_dataset
    xy_splits, xy_test = mock_random_splits_1_dataset

    # -> Files which are supposed to be saved
    files_to_be_saved = [
        ("tpl_x0.npy", xy_splits[0][0], tmp_path),
        ("tpl_y0.npy", xy_splits[0][1], tmp_path),
        ("tpl_x_test.npy", xy_test[0], tmp_path),
        ("tpl_y_test.npy", xy_test[1], tmp_path),
    ]

    files_passed_to_save = []

    def mock_save(
        filename: str,
        data: np.ndarray,
        storage_dir: str = config.get_config("local_dataset_dir"),
    ):
        files_passed_to_save.append((filename, data, storage_dir))

    monkeypatch.setattr(persistence, "save", mock_save)

    # Execute
    persistence.save_splits(
        # filename_template is not relevant for the
        # test as the mock will ignore it
        filename_template=filename_template,
        dataset=mock_random_splits_1_dataset,
        storage_dir=tmp_path,
    )

    # Assert
    for tpl_expected, tpl_actual in zip(files_to_be_saved, files_passed_to_save):
        assert tpl_expected[0] == tpl_actual[0]
        assert tpl_expected[1].shape == tpl_actual[1].shape
        assert tpl_expected[2] == tpl_actual[2]


@pytest.mark.integration
def test_list_files_for_template(mock_datasets_dir):
    """
    Check if we can list files from given directory correctly
    """
    # Prepare
    filename_template = "random_splits_2_{}.npy"

    filenames_expected = [
        "random_splits_2_x0.npy",
        "random_splits_2_y0.npy",
        "random_splits_2_x1.npy",
        "random_splits_2_y1.npy",
        "random_splits_2_x_test.npy",
        "random_splits_2_y_test.npy",
    ]

    # Execute
    filenames_actual = persistence.list_files_for_template(
        filename_template=filename_template, storage_dir=mock_datasets_dir
    )

    # Assert
    assert set(filenames_expected) == set(filenames_actual)
