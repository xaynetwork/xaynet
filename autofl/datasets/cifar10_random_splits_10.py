"""
Easily accessable datasets
"""
import os

import tensorflow as tf

from autofl.data import data, persistence
from autofl.types import FederatedDataset

from .config import get_config

DATASET_NAME = "cifar10_random_splits_10"


def generate_dataset() -> FederatedDataset:
    """Will generate dataset and store it locally"""
    return data.generate_splits(10, tf.keras.datasets.cifar10)


def load_splits():
    return persistence.load_local_dataset(
        dataset_name=DATASET_NAME, local_datasets_dir=get_config("local_datasets_dir")
    )


def load_shard():
    pass


def load_test():
    pass


if __name__ == "__main__":
    """
    Generates and stores dataset locally
    Will only once be used to generate the dataset to be stored online
    """
    dataset = generate_dataset()

    dataset_dir = persistence.get_dataset_dir(
        dataset_name=DATASET_NAME, local_datasets_dir=get_config("local_datasets_dir")
    )

    persistence.save_splits(dataset=dataset, storage_dir=dataset_dir)
