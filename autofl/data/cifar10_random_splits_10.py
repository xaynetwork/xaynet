"""
Easily accessable datasets
"""

import tensorflow as tf

from . import data, persistence
from .typing import FederatedDataset

LOCAL_STORAGE_DIR = "/tmp"
FILENAME_TEMPLATE = "cifar10_random_split_10_{}.npy"


def generate_dataset() -> FederatedDataset:
    return data.load_splits(10, tf.keras.datasets.cifar10)


def store_dataset(dataset: FederatedDataset):
    return persistence.save_splits(filename_template=FILENAME_TEMPLATE, dataset=dataset)


def load_splits():
    # TODO: load generated datasets
    return generate_dataset()
