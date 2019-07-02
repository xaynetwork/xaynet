import numpy as np

from . import cifar10_random_splits_10 as ds
from . import storage


def test_load_split(monkeypatch, tmp_path):
    # Prepare
    split_id = "05"
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

        if split_name == "x_{}.npy".format(split_id):
            return x_expected
        if split_name == "y_{}.npy".format(split_id):
            return y_expected

        raise Exception("split_name was incorrect")

    monkeypatch.setattr(
        storage, "download_remote_ndarray", mock_download_remote_ndarray
    )

    # Execute
    x_actual, y_actual = ds.load_split(split_id, local_datasets_dir=tmp_path)

    # Assert
    np.testing.assert_equal(x_actual, x_actual)
    np.testing.assert_equal(y_actual, y_actual)
