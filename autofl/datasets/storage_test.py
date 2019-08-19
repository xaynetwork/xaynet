from time import time

import numpy
import pytest

from . import storage


@pytest.mark.slow
@pytest.mark.integration
def test_load_ndarray(tmp_path):

    # Prepare
    dataset_name = "integration_test"
    ndarray_name = "x_00.npy"
    ndarray_hash = "6a052c9b5c04e51a84b5ae4b0539707236a979aa"

    ndarray_expected = numpy.ones((3, 2))

    t1 = time() * 1000.0

    # Execute
    ndarray_actual = storage.load_ndarray(
        dataset_name=dataset_name,
        ndarray_name=ndarray_name,
        ndarray_hash=ndarray_hash,
        local_datasets_dir=tmp_path,
    )

    t2 = time() * 1000.0

    # Assert
    # Loading from remote should take less than 10s
    assert (t2 - t1) < 10000
    numpy.testing.assert_equal(ndarray_actual, ndarray_expected)

    # Execute
    ndarray_actual = storage.load_ndarray(
        dataset_name=dataset_name,
        ndarray_name=ndarray_name,
        ndarray_hash=ndarray_hash,
        local_datasets_dir=tmp_path,
    )

    t3 = time() * 1000.0

    # Assert
    # Loading from disk should take less than 10ms
    assert (t3 - t2) < 10
    numpy.testing.assert_equal(ndarray_actual, ndarray_expected)


@pytest.mark.integration
def test_load_ndarray_wrong_hash(tmp_path):
    # Prepare
    dataset_name = "integration_test"
    ndarray_name = "x_00.npy"
    ndarray_hash = "wrong_hash"

    # Execute and expect to fail
    with pytest.raises(Exception):
        storage.load_ndarray(
            dataset_name=dataset_name,
            ndarray_name=ndarray_name,
            ndarray_hash=ndarray_hash,
            local_datasets_dir=tmp_path,
        )
