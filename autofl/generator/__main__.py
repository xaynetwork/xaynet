import tensorflow as tf
from absl import app

from autofl.datasets import (
    cifar10_random_splits_10,
    fashion_mnist_10s_500_1k_bias,
    fashion_mnist_10s_600,
    fashion_mnist_10s_single_class,
    fashion_mnist_100p_IID_balanced,
    fashion_mnist_100p_non_IID,
)
from autofl.generator import data

from .datasets import (
    generate_cifar10_random_splits_10,
    generate_fashion_mnist_10s_500_1k_bias,
    generate_fashion_mnist_10s_600,
    generate_fashion_mnist_10s_single_class,
    generate_fashion_mnist_100p_IID_balanced,
    generate_fashion_mnist_100p_non_IID,
)


def assert_online_datasets():
    data.assert_dataset_origin(
        keras_dataset=data.load(tf.keras.datasets.cifar10),
        federated_dataset=cifar10_random_splits_10.load_splits(),
    )

    fashion_mnist_datasets = [
        fashion_mnist_10s_500_1k_bias,
        fashion_mnist_10s_600,
        fashion_mnist_10s_single_class,
        fashion_mnist_100p_IID_balanced,
        fashion_mnist_100p_non_IID,
    ]

    for ds in fashion_mnist_datasets:
        data.assert_dataset_origin(
            keras_dataset=data.load(tf.keras.datasets.fashion_mnist),
            federated_dataset=ds.load_splits(),
        )


def main(_):
    # assert_online_datasets()

    # Just uncomment the methods you want to run
    generate_cifar10_random_splits_10()
    generate_fashion_mnist_10s_600()
    generate_fashion_mnist_10s_single_class()
    generate_fashion_mnist_10s_500_1k_bias()
    generate_fashion_mnist_100p_IID_balanced()
    generate_fashion_mnist_100p_non_IID()


app.run(main=main)
