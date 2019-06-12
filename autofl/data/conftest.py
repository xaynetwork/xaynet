from typing import Tuple

import pytest
import tensorflow as tf
import numpy as np

from .data import load


class keras_dataset:
    @staticmethod
    def load_data() -> Tuple[
        Tuple[np.ndarray, np.ndarray], Tuple[np.ndarray, np.ndarray]
    ]:
        labels = np.arange(10, dtype=np.int8)

        x_train = np.ones((60, 32, 32, 3), dtype=np.int8)
        y_train = np.tile(labels, 6)

        x_test = np.ones((10, 32, 32, 3), dtype=np.int8)
        y_test = np.tile(labels, 1)

        return (x_train, y_train), (x_test, y_test)


@pytest.fixture
def dataset():
    (x_train, y_train, x_test, y_test) = load(keras_dataset)

    assert isinstance(
        x_train, np.ndarray
    ), "load method in data.py seems to be out of sync with assumptions in fixture"

    assert isinstance(
        y_train, np.ndarray
    ), "load method in data.py seems to be out of sync with assumptions in fixture"

    assert isinstance(
        x_test, np.ndarray
    ), "load method in data.py seems to be out of sync with assumptions in fixture"

    assert isinstance(
        x_test, np.ndarray
    ), "load method in data.py seems to be out of sync with assumptions in fixture"

    return (x_train, y_train, x_test, y_test)
