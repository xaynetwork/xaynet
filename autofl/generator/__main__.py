import tensorflow as tf

from . import config, data, persistence


def generate_cifar10_random_splits_10():
    dataset = data.generate_splits(10, tf.keras.datasets.cifar10)

    persistence.save_splits(
        dataset_name="cifar10_random_splits_10",
        dataset=dataset,
        local_generator_dir=config.local_generator_datasets_dir,
    )


generate_cifar10_random_splits_10()
