import tensorflow as tf
from absl import flags

from xain.datasets import storage
from xain.types import FederatedDataset

FLAGS = flags.FLAGS

cifar10 = tf.keras.datasets.cifar10
fashion_mnist = tf.keras.datasets.fashion_mnist

config = {
    "cifar10_random_splits_10": {"keras_dataset": cifar10},
    "fashion_mnist_100p_01cpp": {"keras_dataset": fashion_mnist},
    "fashion_mnist_100p_02cpp": {"keras_dataset": fashion_mnist},
    "fashion_mnist_100p_03cpp": {"keras_dataset": fashion_mnist},
    "fashion_mnist_100p_04cpp": {"keras_dataset": fashion_mnist},
    "fashion_mnist_100p_05cpp": {"keras_dataset": fashion_mnist},
    "fashion_mnist_100p_06cpp": {"keras_dataset": fashion_mnist},
    "fashion_mnist_100p_07cpp": {"keras_dataset": fashion_mnist},
    "fashion_mnist_100p_08cpp": {"keras_dataset": fashion_mnist},
    "fashion_mnist_100p_09cpp": {"keras_dataset": fashion_mnist},
    "fashion_mnist_100p_10cpp": {"keras_dataset": fashion_mnist},
    "fashion_mnist_100p_IID_balanced": {"keras_dataset": fashion_mnist},
}


def load_splits(
    dataset_name: str, get_local_datasets_dir=storage.default_get_local_datasets_dir
) -> FederatedDataset:
    return storage.load_splits(
        dataset_name=dataset_name, get_local_datasets_dir=get_local_datasets_dir
    )
