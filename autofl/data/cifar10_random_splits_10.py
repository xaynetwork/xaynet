"""
Easily accessable datasets
"""

import tensorflow as tf

from . import data, persistence
from .typing import FederatedDataset

DATASET_NAME = __name__[:-4].split(".")[-1]


def generate_dataset() -> FederatedDataset:
    """Will generate dataset and store it locally"""
    return data.load_splits(10, tf.keras.datasets.cifar10)


def load_splits():
    return persistence.load_or_generate_dataset(
        dataset_name=DATASET_NAME, generate_dataset_method=generate_dataset
    )
