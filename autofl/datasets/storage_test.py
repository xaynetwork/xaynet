import pytest

from . import storage


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
