from typing import Tuple

import tensorflow as tf
from absl import flags

from autofl.datasets import hashes, storage, testing
from autofl.types import FederatedDataset

FLAGS = flags.FLAGS

DATASET_NAME = "fashion_mnist_10s_500_1k_bias"
DATASET_SPLIT_HASHES = hashes.datasets[DATASET_NAME]


def load_splits(
    get_local_datasets_dir=storage.default_get_local_datasets_dir
) -> FederatedDataset:
    ds = storage.load_splits(
        dataset_name=DATASET_NAME,
        dataset_split_hashes=DATASET_SPLIT_HASHES,
        get_local_datasets_dir=get_local_datasets_dir,
    )

    testing.assert_dataset_origin(
        keras_dataset=testing.load(tf.keras.datasets.fashion_mnist),
        federated_dataset=ds,
    )

    return ds


def load_split(
    split_id: str,
    split_hashes: Tuple[str, str],
    get_local_datasets_dir=storage.default_get_local_datasets_dir,
):
    assert split_id in set(DATASET_SPLIT_HASHES.keys())

    x_i, y_i = storage.load_split(
        dataset_name=DATASET_NAME,
        split_id=split_id,
        split_hashes=split_hashes,
        local_datasets_dir=get_local_datasets_dir(),
    )

    return x_i, y_i
