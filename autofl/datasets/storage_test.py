from time import time

import numpy
import pytest

from . import storage


@pytest.mark.integration
def test_download_remote_ndarray(tmp_path, mock_datasets_repository):

    # Prepare
    dataset_name = "integration_test"
    split_name = "ones32.npy"

    ndarray_expected = numpy.ones((3, 2))

    # Execute
    t1 = time() * 1000.0

    ndarray_actual = storage.download_remote_ndarray(
        datasets_repository=mock_datasets_repository,
        dataset_name=dataset_name,
        split_name=split_name,
        local_datasets_dir=tmp_path,
    )

    t2 = time() * 1000.0

    # Loading from remote should take less than 1000ms
    assert (t2 - t1) < 1000

    ndarray_actual = storage.download_remote_ndarray(
        datasets_repository=mock_datasets_repository,
        dataset_name=dataset_name,
        split_name=split_name,
        local_datasets_dir=tmp_path,
    )

    t3 = time() * 1000.0

    # Loading from disk should take less than 10ms
    assert (t3 - t2) < 10

    # Assert
    numpy.testing.assert_equal(ndarray_actual, ndarray_expected)
