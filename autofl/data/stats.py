from typing import Dict, List, Tuple

import numpy as np


def basic_stats(dataset: Tuple[np.ndarray, np.ndarray]) -> Dict:
    """
    Creates dataset statistics for a dataset of the shape:

    "Tuple[ndarray, ndarray]" respectively "(x_train, y_train)"

    Answering the following questions:
      - How many examples
      - How many examples per class
    """

    (x, y) = dataset

    return {
        "number_of_examples": x.shape[0],
        "number_of_examples_per_label": np.unique(y, return_counts=True),
    }


def basic_stats_multiple(datasets: List[Tuple[np.ndarray, np.ndarray]]) -> List[Dict]:
    """
    Creates dataset statistics for multiple datasets which will
    be passed through "basic_stats()" in a loop
    """

    return [basic_stats(dataset) for dataset in datasets]
