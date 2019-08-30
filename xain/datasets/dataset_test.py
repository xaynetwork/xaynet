import random

import numpy as np
import pytest

from .dataset import config, load_splits


@pytest.mark.slow
@pytest.mark.integration
def test_load_splits(tmp_path):
    # Prepare
    def get_local_datasets_dir():
        return tmp_path

    dataset_name = random.choice(list(config))

    # Execute
    xy_splits_actual, xy_val_actual, xy_test_actual = load_splits(
        dataset_name=dataset_name, get_local_datasets_dir=get_local_datasets_dir
    )

    # Assert
    assert isinstance(xy_splits_actual, list)
    assert isinstance(xy_val_actual, tuple)
    assert isinstance(xy_test_actual, tuple)

    for xy in xy_splits_actual:
        x, y = xy

        assert isinstance(x, np.ndarray)
        assert isinstance(y, np.ndarray)


@pytest.mark.integration
def test_load_splits_without_fetch(tmp_path, disable_fetch):  # pylint: disable=W0613
    # Prepare
    dataset_name = list(config)[0]

    def get_local_datasets_dir():
        return tmp_path

    # Execute
    with pytest.raises(Exception):
        load_splits(
            dataset_name=dataset_name, get_local_datasets_dir=get_local_datasets_dir
        )
