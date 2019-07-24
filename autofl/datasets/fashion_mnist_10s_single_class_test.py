import numpy as np
import pytest

from . import fashion_mnist_10s_single_class as fashion10


@pytest.mark.slow
@pytest.mark.integration
def test_load_splits(tmp_path):
    # Prepare
    def get_local_datasets_dir():
        return tmp_path

    # Execute
    xy_splits_actual, xy_validation_actual, xy_test_actual = fashion10.load_splits(
        get_local_datasets_dir=get_local_datasets_dir
    )

    # Assert
    assert isinstance(xy_splits_actual, list)
    assert isinstance(xy_validation_actual, tuple)
    assert isinstance(xy_test_actual, tuple)

    for xy in xy_splits_actual:
        x, y = xy

        assert isinstance(x, np.ndarray)
        assert isinstance(y, np.ndarray)

        # Check that each split contains only one label
        assert len(set(y)) == 1


@pytest.mark.integration
def test_load_splits_without_fetch(tmp_path, disable_fetch):  # pylint: disable=W0613
    # Prepare
    def get_local_datasets_dir():
        return tmp_path

    # Execute
    with pytest.raises(Exception):
        fashion10.load_splits(get_local_datasets_dir=get_local_datasets_dir)
