import numpy as np
import pytest

from autofl.data import persistence


# TODO mark as integration test
# TODO find a better solution for a.npy
@pytest.mark.integration
def test_store_load():
    tmp_file = "/tmp/a.npy"

    # Create NumPy array
    a_expected = np.zeros(shape=(3, 28, 28, 1), dtype=np.uint8)
    a_expected[0][1][1][0] = 255

    # Store to disk, then load from disk
    persistence.store(a_expected, fname=tmp_file)
    a_actual = persistence.load(fname=tmp_file)

    # Test equality
    assert np.array_equal(a_expected, a_actual)
