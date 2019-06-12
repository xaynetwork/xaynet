import os
from typing import Dict, Tuple

import numpy as np

ndarray = np.ndarray


def basic_statistics(dataset: Tuple[ndarray, ndarray, ndarray, ndarray]) -> Dict:
    """
    Creates dataset statistics for a dataset of the shape:
    
    Tuple[ndarray, ndarray, ndarray, ndarray]
    
    respectively

    (x_train, y_train, x_test, y_test)

    Answering the following questions:
      - How many examples
      - How many examples per class
    """

    (x_train, y_train, x_test, y_test) = dataset

    stats = dict()

    stats["train"] = {
        "number_of_examples": x_train.shape[0],
        "number_of_examples_per_label": np.unique(y_train, return_counts=True),
    }

    stats["test"] = {
        "number_of_examples": x_test.shape[0],
        "number_of_examples_per_label": np.unique(y_train, return_counts=True),
    }

    return stats
