import os
from typing import List

import tensorflow as tf

from xain.generator import data, volume_distributions

local_generator_datasets_dir = os.path.expanduser("~/.xain/generator/datasets")

keras_cifar10 = tf.keras.datasets.cifar10
keras_fashion_mnist = tf.keras.datasets.fashion_mnist

# Makes from an int e.g. 5 => 05
leftpad = lambda i: str(i).zfill(2)

cifar10_cpp = {
    **{
        f"cifar-10-100p-noniid-{leftpad(num_cpp)}cpp": {
            "keras_dataset": keras_cifar10,
            "transformers": [data.sorted_labels_sections_shuffle],
            "transformers_kwargs": [{"num_partitions": 100, "cpp": num_cpp}],
            "num_partitions": 100,
            "validation_set_size": 5000,
            "assert_dataset_origin": True,
        }
        for num_cpp in [1, 2, 3, 5, 6, 9]
    },
    # Edge cases where the default parition volume is not
    # divisible by given CPP values
    **{
        f"cifar-10-100p-noniid-{leftpad(num_cpp)}cpp": {
            "keras_dataset": keras_cifar10,
            "transformers": [data.remove_balanced, data.sorted_labels_sections_shuffle],
            "transformers_kwargs": [
                {"num_remove": 200},
                {"num_partitions": 100, "cpp": num_cpp},
            ],
            "num_partitions": 100,
            "validation_set_size": 5000,
            "assert_dataset_origin": False,
        }
        for num_cpp in [4, 7, 8]
    },
    "cifar-10-100p-iid-balanced": {
        "keras_dataset": keras_cifar10,
        "transformers": [data.balanced_labels_shuffle],
        "transformers_kwargs": [{"num_partitions": 100}],
        "num_partitions": 100,
        "validation_set_size": 5000,
        "assert_dataset_origin": True,
    },
}

fashion_mnist_cpp = {
    **{
        f"fashion-mnist-100p-noniid-{leftpad(num_cpp)}cpp": {
            "keras_dataset": keras_fashion_mnist,
            "transformers": [data.sorted_labels_sections_shuffle],
            "transformers_kwargs": [{"num_partitions": 100, "cpp": num_cpp}],
            "num_partitions": 100,
            "validation_set_size": 6000,
            "assert_dataset_origin": True,
        }
        # num_cpp 7 and 8 are special cases; see next
        for num_cpp in [1, 2, 3, 4, 5, 6, 9]
    },
    # Edge cases where the default parition volume is not
    # divisible by given CPP values
    "fashion-mnist-100p-noniid-07cpp": {
        "keras_dataset": keras_fashion_mnist,
        "transformers": [data.remove_balanced, data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [
            # we need to remove 100 elements from the full xy_train so the
            # 540 examples per partition are reduced to 539 and therefore
            # divisible by 7
            {"num_remove": 100},
            {"num_partitions": 100, "cpp": 7},
        ],
        "num_partitions": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": False,
    },
    "fashion-mnist-100p-noniid-08cpp": {
        "keras_dataset": keras_fashion_mnist,
        "transformers": [data.remove_balanced, data.sorted_labels_sections_shuffle],
        "transformers_kwargs": [
            # we need to remove 400 elements from the full xy_train so the
            # 540 examples per partition are reduced to 536 and therefore
            # divisible by 8
            {"num_remove": 400},
            {"num_partitions": 100, "cpp": 8},
        ],
        "num_partitions": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": False,
    },
    "fashion-mnist-100p-iid-balanced": {
        "keras_dataset": keras_fashion_mnist,
        "transformers": [data.balanced_labels_shuffle],
        "transformers_kwargs": [{"num_partitions": 100}],
        "num_partitions": 100,
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    },
}


def b_to_str(b: float):
    b_str = f"{b:<f}"
    return b_str[:5].replace(".", "_")


def dist_to_indicies(dist: List[int]) -> List[int]:
    indices = [0] * len(dist)
    for i, _ in enumerate(dist):
        if i == 0:
            indices[i] = dist[i]
        else:
            indices[i] = indices[i - 1] + dist[i]

    assert indices[-1] == sum(dist)

    # Exclude last element as indices only mark start of section
    return indices[:-1]


cifar10_volumes = {
    # FIXME str(i) should always have 3 nums after decimal
    f"cifar-10-100p-b{b_to_str(b)}": {
        "keras_dataset": keras_cifar10,
        "transformers": [data.random_shuffle],
        "transformers_kwargs": {},
        "num_partitions": dist_to_indicies(dist),
        "validation_set_size": 5000,
        "assert_dataset_origin": True,
    }
    for b, dist in volume_distributions.cifar_10_100p()
}

fashion_mnist_volumes = {
    f"fashion-mnist-100p-b{b_to_str(b)}": {
        "keras_dataset": keras_fashion_mnist,
        "transformers": [data.random_shuffle],
        "transformers_kwargs": {},
        "num_partitions": dist_to_indicies(dist),
        "validation_set_size": 6000,
        "assert_dataset_origin": True,
    }
    for b, dist in volume_distributions.fashion_mnist_100p()
}


datasets = {
    **cifar10_cpp,
    **fashion_mnist_cpp,
    **cifar10_volumes,
    **fashion_mnist_volumes,
}
