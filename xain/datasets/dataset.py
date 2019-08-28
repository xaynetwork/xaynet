import tensorflow as tf
from absl import flags

from xain.datasets import storage
from xain.types import FederatedDataset

FLAGS = flags.FLAGS

cifar10 = tf.keras.datasets.cifar10
fashion_mnist = tf.keras.datasets.fashion_mnist

config = {
    "cifar-10-100p-noniid-01cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-02cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-03cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-04cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-05cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-06cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-07cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-08cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-09cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-10cpp": {"keras_dataset": cifar10},
    "fashion-mnist-100p-noniid-01cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-02cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-03cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-04cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-05cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-06cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-07cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-08cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-09cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-10cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-iid-balanced": {"keras_dataset": fashion_mnist},
}


def load_splits(
    dataset_name: str, get_local_datasets_dir=storage.default_get_local_datasets_dir
) -> FederatedDataset:
    return storage.load_splits(
        dataset_name=dataset_name, get_local_datasets_dir=get_local_datasets_dir
    )
