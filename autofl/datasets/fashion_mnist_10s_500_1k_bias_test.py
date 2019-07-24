import numpy as np
import pytest

from . import fashion_mnist_10s_500_1k_bias as fashion10


@pytest.mark.slow
@pytest.mark.integration
def test_load_splits(tmp_path):
    # Prepare
    def get_local_datasets_dir():
        return tmp_path

    # Execute
    xy_splits_actual, xy_val_actual, xy_test_actual = fashion10.load_splits(
        get_local_datasets_dir=get_local_datasets_dir
    )

    # Assert
    assert isinstance(xy_splits_actual, list)
    assert isinstance(xy_val_actual, tuple)
    assert isinstance(xy_test_actual, tuple)

    for xy in xy_splits_actual:
        x, y = xy

        assert isinstance(x, np.ndarray)
        assert isinstance(y, np.ndarray)

        counts = np.unique(np.unique(y, return_counts=True)[1], return_counts=True)
        # we should have one label which occurs 1500 times
        # and 9 labels which occur 500 times
        assert set(counts[0]) == set([1500, 500])
        assert set(counts[1]) == set([1, 9])


@pytest.mark.integration
def test_load_splits_without_fetch(tmp_path, disable_fetch):  # pylint: disable=W0613
    # Prepare
    def get_local_datasets_dir():
        return tmp_path

    # Execute
    with pytest.raises(Exception):
        fashion10.load_splits(get_local_datasets_dir=get_local_datasets_dir)
