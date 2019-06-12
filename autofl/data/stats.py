from typing import List, Tuple

import numpy as np


class DatasetStats:
    """
    Inheriting from Mapping makes DatasetStats immutable after instantiation
    See: https://docs.python.org/3/library/collections.abc.html
    """

    def __init__(self, number_of_examples: int, number_of_examples_per_label: tuple):
        self.number_of_examples = number_of_examples
        self.number_of_examples_per_label = number_of_examples_per_label

        assert isinstance(
            number_of_examples_per_label, tuple
        ), "property number_of_examples_per_label should be a tuple"


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


def basic_stats_multiple(
    datasets: List[Tuple[np.ndarray, np.ndarray]]
) -> List[DatasetStats]:
    """
    Creates dataset statistics for multiple datasets which will
    be passed through "basic_stats()" in a loop
    """

    return [basic_stats(dataset) for dataset in datasets]
