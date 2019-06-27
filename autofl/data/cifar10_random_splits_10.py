"""
Easily accessable datasets
"""

import tensorflow as tf

from . import data
from .typing import FederatedDataset

DATASET_NAME = __name__[:-4]

print(DATASET_NAME)


def generate_dataset() -> FederatedDataset:
    return data.load_splits(10, tf.keras.datasets.cifar10)


def store_dataset():
    """Generates and stores dataset locally"""
    # TODO: implement


def load_splits():
    # TODO: implement properly
    return generate_dataset()
