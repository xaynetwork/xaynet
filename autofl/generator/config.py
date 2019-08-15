import os

import tensorflow as tf

from autofl.generator import data

local_generator_datasets_dir = os.path.expanduser("~/.autofl/generator/datasets")

datasets = {
    # "cifar10_random_splits_10": {
    #     "keras_dataset": tf.keras.datasets.cifar10,
    #     "transformer": data.random_shuffle,
    #     "transformer_kwargs": {},
    #     "num_splits": 10,
    #     "validation_set_size": 5000,
    # },
    # "fashion_mnist_10s_600": {
    #     "keras_dataset": tf.keras.datasets.fashion_mnist,
    #     "transformer": data.balanced_labels_shuffle,
    #     "transformer_kwargs": {"num_partitions": 10},
    #     "num_splits": 10,
    #     "validation_set_size": 6000,
    # },
    # "fashion_mnist_10s_500_1k_bias": {
    #     "keras_dataset": tf.keras.datasets.fashion_mnist,
    #     "transformer": data.biased_balanced_labels_shuffle,
    #     "transformer_kwargs": {"bias": 1000},
    #     "num_splits": 10,
    #     "validation_set_size": 6000,
    # },
    # "fashion_mnist_10s_single_class": {
    #     "keras_dataset": tf.keras.datasets.fashion_mnist,
    #     "transformer": data.group_by_label,
    #     "transformer_kwargs": {},
    #     "num_splits": 10,
    #     "validation_set_size": 6000,
    # },
    # "fashion_mnist_100p_IID_balanced": {
    #     "keras_dataset": tf.keras.datasets.fashion_mnist,
    #     "transformer": data.balanced_labels_shuffle,
    #     "transformer_kwargs": {"num_partitions": 100},
    #     "num_splits": 100,
    #     "validation_set_size": 6000,
    # },
    # "fashion_mnist_100p_non_IID": {
    #     "keras_dataset": tf.keras.datasets.fashion_mnist,
    #     "transformer": data.sorted_labels_sections_shuffle,
    #     "transformer_kwargs": {"num_partitions": 100},
    #     "num_splits": 100,
    #     "validation_set_size": 6000,
    # },
    "fashion_mnist_100p_01cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.sorted_labels_sections_shuffle,
        "transformer_kwargs": {"num_partitions": 100, "class_per_partition": 1},
        "num_splits": 100,
        "validation_set_size": 6000,
    },
    "fashion_mnist_100p_02cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.sorted_labels_sections_shuffle,
        "transformer_kwargs": {"num_partitions": 100, "class_per_partition": 2},
        "num_splits": 100,
        "validation_set_size": 6000,
    },
    "fashion_mnist_100p_03cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.sorted_labels_sections_shuffle,
        "transformer_kwargs": {"num_partitions": 100, "class_per_partition": 3},
        "num_splits": 100,
        "validation_set_size": 6000,
    },
    "fashion_mnist_100p_04cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.sorted_labels_sections_shuffle,
        "transformer_kwargs": {"num_partitions": 100, "class_per_partition": 4},
        "num_splits": 100,
        "validation_set_size": 6000,
    },
    "fashion_mnist_100p_05cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.sorted_labels_sections_shuffle,
        "transformer_kwargs": {"num_partitions": 100, "class_per_partition": 5},
        "num_splits": 100,
        "validation_set_size": 6000,
    },
    "fashion_mnist_100p_06cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.sorted_labels_sections_shuffle,
        "transformer_kwargs": {"num_partitions": 100, "class_per_partition": 6},
        "num_splits": 100,
        "validation_set_size": 6000,
    },
    "fashion_mnist_100p_07cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.sorted_labels_sections_shuffle,
        "transformer_kwargs": {"num_partitions": 100, "class_per_partition": 7},
        "num_splits": 100,
        # 56000k will be in x_train with a x_val of 4k and therefore 560 per
        # partition for 100 partitions. 560 is dividable by 8 opposed to
        # 540 with a validation_set_size of 6k as in most other similar datasets
        "validation_set_size": 4000,
    },
    "fashion_mnist_100p_08cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.sorted_labels_sections_shuffle,
        "transformer_kwargs": {"num_partitions": 100, "class_per_partition": 8},
        "num_splits": 100,
        # 56000k will be in x_train with a x_val of 4k and therefore 560 per
        # partition for 100 partitions. 560 is dividable by 8 opposed to
        # 540 with a validation_set_size of 6k as in most other similar datasets
        "validation_set_size": 4000,
    },
    "fashion_mnist_100p_09cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.sorted_labels_sections_shuffle,
        "transformer_kwargs": {"num_partitions": 100, "class_per_partition": 9},
        "num_splits": 100,
        "validation_set_size": 6000,
    },
    "fashion_mnist_100p_10cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformer": data.sorted_labels_sections_shuffle,
        "transformer_kwargs": {"num_partitions": 100, "class_per_partition": 10},
        "num_splits": 100,
        "validation_set_size": 6000,
    },
}
