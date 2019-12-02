import tensorflow as tf
from absl import flags

from xain_fl.datasets import storage
from xain_fl.types import FederatedDataset

FLAGS = flags.FLAGS

cifar10 = tf.keras.datasets.cifar10
fashion_mnist = tf.keras.datasets.fashion_mnist

config = {
    # cpp datasets
    "cifar-10-100p-noniid-01cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-02cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-03cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-04cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-05cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-06cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-07cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-08cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-noniid-09cpp": {"keras_dataset": cifar10},
    "cifar-10-100p-iid-balanced": {"keras_dataset": cifar10},
    "fashion-mnist-100p-noniid-01cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-02cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-03cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-04cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-05cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-06cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-07cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-08cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-noniid-09cpp": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-iid-balanced": {"keras_dataset": fashion_mnist},
    # volume based datasets
    "cifar-10-100p-b1_000": {"keras_dataset": cifar10},
    "cifar-10-100p-b1_005": {"keras_dataset": cifar10},
    "cifar-10-100p-b1_010": {"keras_dataset": cifar10},
    "cifar-10-100p-b1_015": {"keras_dataset": cifar10},
    "cifar-10-100p-b1_020": {"keras_dataset": cifar10},
    "cifar-10-100p-b1_025": {"keras_dataset": cifar10},
    "cifar-10-100p-b1_030": {"keras_dataset": cifar10},
    "cifar-10-100p-b1_035": {"keras_dataset": cifar10},
    "cifar-10-100p-b1_040": {"keras_dataset": cifar10},
    "cifar-10-100p-b1_045": {"keras_dataset": cifar10},
    "fashion-mnist-100p-b1_000": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-b1_005": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-b1_010": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-b1_015": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-b1_020": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-b1_025": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-b1_030": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-b1_035": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-b1_040": {"keras_dataset": fashion_mnist},
    "fashion-mnist-100p-b1_045": {"keras_dataset": fashion_mnist},
}


def load_splits(
    dataset_name: str, get_local_datasets_dir=storage.default_get_local_datasets_dir
) -> FederatedDataset:
    """Will load and return federated dataset

        Args:
            dataset_name (str): Name of dataset to be loaded. Valid names can be found
                                in xain_fl.datasets.dataset.config dict
            get_local_datasets_dir (Callable): Function which returns the local_datasets_dir

        Returns:
            FederatedDataset
    """
    return storage.load_splits(
        dataset_name=dataset_name, get_local_datasets_dir=get_local_datasets_dir
    )
