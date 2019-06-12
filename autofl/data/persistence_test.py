import numpy as np

from autofl.data import persistence


# TODO mark as integration test
# TODO find a better solution for a.npy
def test_store_load():
    fname = "a"
    fname_full = "a.npy"

    # Create NumPy array
    a_expected = np.zeros(shape=(3, 28, 28, 1), dtype=np.uint8)
    a_expected[0][1][1][0] = 255

    # Store to disk, then load from disk
    persistence.store(a_expected, fname=fname)
    a_actual = persistence.load(fname=fname_full)

    # Test equality
    assert np.array_equal(a_expected, a_actual)
