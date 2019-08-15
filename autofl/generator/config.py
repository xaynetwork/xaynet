import os

import tensorflow as tf

from autofl.generator import data

local_generator_datasets_dir = os.path.expanduser("~/.autofl/generator/datasets")

datasets = {
    "cifar10_random_splits_10": {
        "keras_dataset": tf.keras.datasets.cifar10,
        "transformer": data.random_shuffle,
        "transformer_kwargs": {},
        "num_splits": 10,
        "validation_set_size": 5000,
    },
    "fashion_mnist_10s_600": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.balanced_labels_shuffle,
        "transformer_kwargs": {"num_partitions": 10},
        "num_splits": 10,
        "validation_set_size": 6000,
    },
    "fashion_mnist_10s_500_1k_bias": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.biased_balanced_labels_shuffle,
        "transformer_kwargs": {"bias": 1000},
        "num_splits": 10,
        "validation_set_size": 6000,
    },
    "fashion_mnist_10s_single_class": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.group_by_label,
        "transformer_kwargs": {},
        "num_splits": 10,
        "validation_set_size": 6000,
    },
    "fashion_mnist_100p_IID_balanced": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.balanced_labels_shuffle,
        "transformer_kwargs": {"num_partitions": 100},
        "num_splits": 100,
        "validation_set_size": 6000,
    },
    "fashion_mnist_100p_non_IID": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.sorted_labels_sections_shuffle,
        "transformer_kwargs": {"num_partitions": 100},
        "num_splits": 100,
        "validation_set_size": 6000,
    },
}
