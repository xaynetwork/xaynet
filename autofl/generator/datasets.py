import tensorflow as tf

from . import config, data, persistence


def generate_cifar10_random_splits_10():
    dataset = data.generate_splits(
        num_splits=10,
        keras_dataset=tf.keras.datasets.cifar10,
        shuffle_method=data.random_shuffle,
    )

    persistence.save_splits(
        dataset_name="cifar10_random_splits_10",
        dataset=dataset,
        local_generator_dir=config.local_generator_datasets_dir,
    )


def generate_fashion_mnist_10s_600():
    dataset = data.generate_splits(
        num_splits=10,
        keras_dataset=tf.keras.datasets.fashion_mnist,
        shuffle_method=data.balanced_classes_shuffle,
    )

    persistence.save_splits(
        dataset_name="fashion_mnist_10s_600",
        dataset=dataset,
        local_generator_dir=config.local_generator_datasets_dir,
    )
