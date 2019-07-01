import numpy as np
import tensorflow as tf

from autofl.datasets import cifar10_random_splits_10


def test_generate_dataset(mock_keras_dataset, monkeypatch):
    # Prepare
    monkeypatch.setattr(tf.keras.datasets, "cifar10", mock_keras_dataset)

    # Execute
    xy_splits, xy_test = cifar10_random_splits_10.generate_dataset()

    # Assert
    # -> Verify types
    assert isinstance(xy_splits, list)
    assert isinstance(xy_test, tuple)
    assert len(xy_splits) == 10

    # -> Test shape of splits
    for split in xy_splits:
        x, y = split

        assert isinstance(x, np.ndarray)
        assert isinstance(y, np.ndarray)

        assert x.shape == (6, 32, 32, 3)
        assert y.shape == (6,)

    # -> Check test set
    x_test, y_test = xy_test

    assert x_test.shape == (10, 32, 32, 3)
    assert y_test.shape == (10,)


def test_store_dataset():
    # TODO
    # Prepare
    # Execute
    # Assert
    pass
