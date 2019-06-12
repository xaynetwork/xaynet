from typing import Tuple

import numpy as np
import pytest

from autofl.data import data


class KerasDataset:  # pylint: disable=too-few-public-methods
    """
    Used as a mock dataset which will go through the load method in the data.py module
    to make sure that the mock dataset stays compatible with the default load function
    for all datasets in the project
    """

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
    return data.load(KerasDataset())
