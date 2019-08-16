import os

import tensorflow as tf

from autofl.generator import data

local_generator_datasets_dir = os.path.expanduser("~/.autofl/generator/datasets")

datasets = {
    "cifar10_random_splits_10": {
        "keras_dataset": tf.keras.datasets.cifar10,
        "transformers": [data.random_shuffle],
        "transformers_kwargs": [{}],
        "num_splits": 10,
        "validation_set_size": 5000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_10s_600": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.balanced_labels_shuffle],
        "transformers_kwargs": [{"num_partitions": 10}],
        "num_splits": 10,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_10s_500_1k_bias": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.biased_balanced_labels_shuffle],
        "transformers_kwargs": [{"bias": 1000}],
        "num_splits": 10,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_10s_single_class": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.group_by_label],
        "transformers_kwargs": [{}],
        "num_splits": 10,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_100p_IID_balanced": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.balanced_labels_shuffle],
        "transformers_kwargs": [{"num_partitions": 100}],
        "num_splits": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_100p_01cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [{"num_partitions": 100, "class_per_partition": 1}],
        "num_splits": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_100p_02cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [{"num_partitions": 100, "class_per_partition": 2}],
        "num_splits": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_100p_03cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [{"num_partitions": 100, "class_per_partition": 3}],
        "num_splits": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_100p_04cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [{"num_partitions": 100, "class_per_partition": 4}],
        "num_splits": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_100p_05cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [{"num_partitions": 100, "class_per_partition": 5}],
        "num_splits": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_100p_06cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [{"num_partitions": 100, "class_per_partition": 6}],
        "num_splits": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_100p_07cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.take_balanced, data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [
            # we need to remove 100 elements from the full xy_train so the
            # 540 examples per partition are reduced to 539 and therefore
            # divideable by 7
            {"num_take": 100},
            {"num_partitions": 100, "class_per_partition": 7},
        ],
        "num_splits": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": False,
    },
    "fashion_mnist_100p_08cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.take_balanced, data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [
            # we need to remove 400 elements from the full xy_train so the
            # 540 examples per partition are reduced to 536 and therefore
            # divideable by 8
            {"num_take": 400},
            {"num_partitions": 100, "class_per_partition": 8},
        ],
        "num_splits": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": False,
    },
    "fashion_mnist_100p_09cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [{"num_partitions": 100, "class_per_partition": 9}],
        "num_splits": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
    "fashion_mnist_100p_10cpp": {
        "keras_dataset": tf.keras.datasets.fashion_mnist,
        "transformers": [data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [{"num_partitions": 100, "class_per_partition": 10}],
        "num_splits": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
}
