import tensorflow as tf

from . import config, data, persistence


def generate_cifar10_random_splits_10():
    dataset = data.generate_splits(10, tf.keras.datasets.cifar10)

    dataset_dir = persistence.get_generator_dataset_dir(
        dataset_name="cifar10_random_splits_10",
        local_generator_dir=config.local_generator_datasets_dir,
    )

    persistence.save_splits(dataset=dataset, storage_dir=dataset_dir)


generate_cifar10_random_splits_10()
