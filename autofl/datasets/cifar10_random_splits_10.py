from typing import Tuple

import tensorflow as tf
from absl import flags

from autofl.datasets import storage, testing
from autofl.types import FederatedDataset

FLAGS = flags.FLAGS

DATASET_NAME = "cifar10_random_splits_10"
DATASET_SPLIT_HASHES = {
    "00": [
        "43a78a026af3580a789492828146ef3909b26358",
        "b8bccc5ca821dcc1a8fbbd4f5e114d736dee17b5",
    ],
    "01": [
        "06aff61968f4a3761c50464442f38741c9361285",
        "bdbf4220549ad0bc8727b20d580cc8ba13bd5512",
    ],
    "02": [
        "a909194ddf9fd3de7e008a34715014e343aa7fb6",
        "eaad57383afc62015fd2e7c96f66bf0230db0bb1",
    ],
    "03": [
        "8b80af6cb91a9696c8a548afbe5952f02d025d72",
        "4a149bd2d3b841c3364c08c8622f39ca2d094fec",
    ],
    "04": [
        "49db375125cace3663343e7c3f877c836fb195da",
        "56678b6f40b5b7132e2753059b386c41a7fe54bd",
    ],
    "05": [
        "03099f56893cdf462521b7731b37a8b8763690a3",
        "926d036ec0f64348d33949737113995a564c0f47",
    ],
    "06": [
        "9490f51359c9c018a568ae257b24639977f9942b",
        "3d060b31b6fce60d1819309ada6ce0490aa70d1e",
    ],
    "07": [
        "0794a3874f7640ee3b4869d4861e48d2c881122a",
        "f697694081b6631f6e12c50e16762e963658cea7",
    ],
    "08": [
        "b07dc54826bc54eac9a41294de432bc7e735092c",
        "8f5e58a30e23e3590598c1fadd2be440f86a274f",
    ],
    "09": [
        "2d02941bc2c2392e10334c2c5489b3216a6bd8c4",
        "9975f84e1369634ce817e6bacfa24281b5c94544",
    ],
    "test": [
        "b3445026a7109c894048d592569b09c8344d167e",
        "44127f76beca2a38125f22debb2c6d92631e4080",
    ],
    "val": [
        "de6bab29c49dfce34c389da4c791f4c0b276cbea",
        "faaa5f5d3329dc4e961fc0f7dfaa3c1266fccb2f",
    ],
}


def load_splits(
    get_local_datasets_dir=storage.default_get_local_datasets_dir
) -> FederatedDataset:
    ds = storage.load_splits(
        dataset_name=DATASET_NAME,
        dataset_split_hashes=DATASET_SPLIT_HASHES,
        get_local_datasets_dir=get_local_datasets_dir,
    )

    testing.assert_dataset_origin(
        keras_dataset=testing.load(tf.keras.datasets.cifar10), federated_dataset=ds
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
