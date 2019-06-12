from collections.abc import Mapping
from typing import Dict, List, Tuple

import numpy as np


class DatasetStats(Mapping):
    """
    Inheriting from Mapping makes DatasetStats immutable after instantiation
    See: https://docs.python.org/3/library/collections.abc.html
    """

    def __init__(self, *args, **kw):
        # Make certain properties required
        assert "number_of_examples" in kw, "property number_of_examples is required"
        assert (
            "number_of_examples_per_label" in kw
        ), "property number_of_examples_per_label is required"

        self._storage = dict(*args, **kw)

    def __getitem__(self, key):
        return self._storage[key]

    def __iter__(self):
        return iter(self._storage)

    def __len__(self):
        return len(self._storage)


def basic_stats(dataset: Tuple[np.ndarray, np.ndarray]) -> DatasetStats:
    """
    Creates dataset statistics for a dataset of the shape:

    "Tuple[ndarray, ndarray]" respectively "(x_train, y_train)"

    Answering the following questions:
      - How many examples
      - How many examples per class
    """

    (x, y) = dataset

    return DatasetStats(
        number_of_examples=x.shape[0],
        number_of_examples_per_label=np.unique(y, return_counts=True),
    )


def basic_stats_multiple(datasets: List[Tuple[np.ndarray, np.ndarray]]) -> List[Dict]:
    """
    Creates dataset statistics for multiple datasets which will
    be passed through "basic_stats()" in a loop
    """

    return [basic_stats(dataset) for dataset in datasets]
